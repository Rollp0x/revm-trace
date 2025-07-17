//! Multicall utilities for batch contract calls
//! by dynamically deploying a Multicall contract and executing batch calls.
//!
//! Key features:
//! - Works on any EVM chain (no need for pre-deployed Multicall contracts)
//! - Dynamically deploys Multicall contract in simulation
//! - Supports batch calls with individual error handling
//! - Lightweight implementation without complex inspectors
//!
//! This module provides a universal Multicall solution that works on any EVM chain.

use alloy::{
    hex,
    primitives::{Address, Bytes, TxKind},
    sol_types::SolCall,
};
use anyhow::Result;
use revm::{
    context::TxEnv,
    context_interface::result::ExecutionResult,
    database::{CacheDB, Database, DatabaseCommit, DatabaseRef},
    ExecuteCommitEvm, ExecuteEvm,
};

use crate::{
    errors::{EvmError, RuntimeError},
    evm::TraceEvm,
    traits::ResetDB,
};

// Multicall3 interface - standard and widely supported
mod multicall3 {
    use alloy::sol;

    sol! {
        #[derive(Debug)]
        struct MulticallCall {
            address target;
            bytes callData;
        }

        #[derive(Debug)]
        struct MulticallResult {
            bool success;
            bytes returnData;
        }

        contract Multicall3 {
            function aggregate(MulticallCall[] calldata calls)
                public payable
                returns (uint256 blockNumber, bytes[] memory returnData);

            function tryAggregate(bool requireSuccess, MulticallCall[] calldata calls)
                public payable
                returns (MulticallResult[] memory returnData);
        }
    }
}

use multicall3::Multicall3::tryAggregateCall;
pub use multicall3::{MulticallCall, MulticallResult};

/// Multicall manager for batch contract calls
///
/// Manages the deployment and execution of Multicall contracts for batch operations.
/// This allows executing multiple contract calls in a single transaction, improving
/// efficiency and reducing the number of required RPC calls.
pub struct MulticallManager {
    /// Multicall3 contract bytecode for deployment
    multicall_bytecode: Bytes,
}

impl MulticallManager {
    /// Create a new MulticallManager with default Multicall3 bytecode
    ///
    /// Initializes the manager with a simplified Multicall contract that can handle
    /// basic multi-call operations. In production environments, you may want to
    /// use the full Multicall3 contract bytecode.
    ///
    /// # Returns
    /// A new `MulticallManager` instance ready for deployment and execution
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use revm_trace::{create_evm, utils::multicall_utils::MulticallManager};
    ///
    /// let evm = create_evm("https://eth.llamarpc.com").await?;
    /// let manager = MulticallManager::new();
    /// // Use manager.deploy_and_batch_call() to execute multiple calls
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Self {
        // Multicall3 bytecode - this is a simplified version for testing
        // In production, you'd want to use the full Multicall3 contract
        // This contract provides basic multicall functionality with tryAggregate support
        const SIMPLE_MULTICALL_BYTECODE: &str = "0x608060405234801561001057600080fd5b50610ee0806100206000396000f3fe6080604052600436106100f35760003560e01c80634d2301cc1161008a578063a8b0574e11610059578063a8b0574e1461025a578063bce38bd714610275578063c3077fa914610288578063ee82ac5e1461029b57600080fd5b80634d2301cc146101ec57806372425d9d1461022157806382ad56cb1461023457806386d516e81461024757600080fd5b80633408e470116100c65780633408e47014610191578063399542e9146101a45780633e64a696146101c657806342cbb15c146101d957600080fd5b80630f28c97d146100f8578063174dea711461011a578063252dba421461013a57806327e86d6e1461015b575b600080fd5b34801561010457600080fd5b50425b6040519081526020015b60405180910390f35b61012d610128366004610a85565b6102ba565b6040516101119190610bbe565b61014d610148366004610a85565b6104ef565b604051610111929190610bd8565b34801561016757600080fd5b50437fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0140610107565b34801561019d57600080fd5b5046610107565b6101b76101b2366004610c60565b610690565b60405161011193929190610cba565b3480156101d257600080fd5b5048610107565b3480156101e557600080fd5b5043610107565b3480156101f857600080fd5b50610107610207366004610ce2565b73ffffffffffffffffffffffffffffffffffffffff163190565b34801561022d57600080fd5b5044610107565b61012d610242366004610a85565b6106ab565b34801561025357600080fd5b5045610107565b34801561026657600080fd5b50604051418152602001610111565b61012d610283366004610c60565b61085a565b6101b7610296366004610a85565b610a1a565b3480156102a757600080fd5b506101076102b6366004610d18565b4090565b60606000828067ffffffffffffffff8111156102d8576102d8610d31565b60405190808252806020026020018201604052801561031e57816020015b6040805180820190915260008152606060208201528152602001906001900390816102f65790505b5092503660005b8281101561047757600085828151811061034157610341610d60565b6020026020010151905087878381811061035d5761035d610d60565b905060200281019061036f9190610d8f565b6040810135958601959093506103886020850185610ce2565b73ffffffffffffffffffffffffffffffffffffffff16816103ac6060870187610dcd565b6040516103ba929190610e32565b60006040518083038185875af1925050503d80600081146103f7576040519150601f19603f3d011682016040523d82523d6000602084013e6103fc565b606091505b50602080850191909152901515808452908501351761046d577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260846000fd5b5050600101610325565b508234146104e6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601a60248201527f4d756c746963616c6c333a2076616c7565206d69736d6174636800000000000060448201526064015b60405180910390fd5b50505092915050565b436060828067ffffffffffffffff81111561050c5761050c610d31565b60405190808252806020026020018201604052801561053f57816020015b606081526020019060019003908161052a5790505b5091503660005b8281101561068657600087878381811061056257610562610d60565b90506020028101906105749190610e42565b92506105836020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166105a66020850185610dcd565b6040516105b4929190610e32565b6000604051808303816000865af19150503d80600081146105f1576040519150601f19603f3d011682016040523d82523d6000602084013e6105f6565b606091505b5086848151811061060957610609610d60565b602090810291909101015290508061067d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b50600101610546565b5050509250929050565b43804060606106a086868661085a565b905093509350939050565b6060818067ffffffffffffffff8111156106c7576106c7610d31565b60405190808252806020026020018201604052801561070d57816020015b6040805180820190915260008152606060208201528152602001906001900390816106e55790505b5091503660005b828110156104e657600084828151811061073057610730610d60565b6020026020010151905086868381811061074c5761074c610d60565b905060200281019061075e9190610e76565b925061076d6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff166107906040850185610dcd565b60405161079e929190610e32565b6000604051808303816000865af19150503d80600081146107db576040519150601f19603f3d011682016040523d82523d6000602084013e6107e0565b606091505b506020808401919091529015158083529084013517610851577f08c379a000000000000000000000000000000000000000000000000000000000600052602060045260176024527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060445260646000fd5b50600101610714565b6060818067ffffffffffffffff81111561087657610876610d31565b6040519080825280602002602001820160405280156108bc57816020015b6040805180820190915260008152606060208201528152602001906001900390816108945790505b5091503660005b82811015610a105760008482815181106108df576108df610d60565b602002602001015190508686838181106108fb576108fb610d60565b905060200281019061090d9190610e42565b925061091c6020840184610ce2565b73ffffffffffffffffffffffffffffffffffffffff1661093f6020850185610dcd565b60405161094d929190610e32565b6000604051808303816000865af19150503d806000811461098a576040519150601f19603f3d011682016040523d82523d6000602084013e61098f565b606091505b506020830152151581528715610a07578051610a07576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152601760248201527f4d756c746963616c6c333a2063616c6c206661696c656400000000000000000060448201526064016104dd565b506001016108c3565b5050509392505050565b6000806060610a2b60018686610690565b919790965090945092505050565b60008083601f840112610a4b57600080fd5b50813567ffffffffffffffff811115610a6357600080fd5b6020830191508360208260051b8501011115610a7e57600080fd5b9250929050565b60008060208385031215610a9857600080fd5b823567ffffffffffffffff811115610aaf57600080fd5b610abb85828601610a39565b90969095509350505050565b6000815180845260005b81811015610aed57602081850181015186830182015201610ad1565b81811115610aff576000602083870101525b50601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169290920160200192915050565b600082825180855260208086019550808260051b84010181860160005b84811015610bb1578583037fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe001895281518051151584528401516040858501819052610b9d81860183610ac7565b9a86019a9450505090830190600101610b4f565b5090979650505050505050565b602081526000610bd16020830184610b32565b9392505050565b600060408201848352602060408185015281855180845260608601915060608160051b870101935082870160005b82811015610c52577fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa0888703018452610c40868351610ac7565b95509284019290840190600101610c06565b509398975050505050505050565b600080600060408486031215610c7557600080fd5b83358015158114610c8557600080fd5b9250602084013567ffffffffffffffff811115610ca157600080fd5b610cad86828701610a39565b9497909650939450505050565b838152826020820152606060408201526000610cd96060830184610b32565b95945050505050565b600060208284031215610cf457600080fd5b813573ffffffffffffffffffffffffffffffffffffffff81168114610bd157600080fd5b600060208284031215610d2a57600080fd5b5035919050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052604160045260246000fd5b7f4e487b7100000000000000000000000000000000000000000000000000000000600052603260045260246000fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff81833603018112610dc357600080fd5b9190910192915050565b60008083357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe1843603018112610e0257600080fd5b83018035915067ffffffffffffffff821115610e1d57600080fd5b602001915036819003821315610a7e57600080fd5b8183823760009101908152919050565b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffc1833603018112610dc357600080fd5b600082357fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffa1833603018112610dc357600080fdfea2646970667358221220bb2b5c71a328032f97c676ae39a1ec2148d3e5d6f73d95e9b17910152d61f16264736f6c634300080c0033";

        Self {
            multicall_bytecode: Bytes::from(hex::decode(SIMPLE_MULTICALL_BYTECODE).unwrap()),
        }
    }

    /// Deploy a Multicall contract to the EVM state
    ///
    /// This method deploys a simple Multicall3-compatible contract that can execute
    /// multiple contract calls in a single transaction. The deployment uses a CREATE
    /// transaction and returns the deployed contract address.
    ///
    /// # Arguments
    /// * `evm` - Mutable reference to the EVM instance
    ///
    /// # Returns
    /// * `Ok(Address)` - Address of the deployed Multicall contract
    /// * `Err(EvmError)` - If deployment fails or contract creation is invalid
    ///
    /// # Implementation Details
    /// - Uses CREATE transaction type to deploy the contract
    /// - Returns the contract address from the execution output
    /// - Handles revert and halt scenarios appropriately
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use revm_trace::{create_evm, utils::multicall_utils::MulticallManager};
    ///
    /// let mut evm = create_evm("https://eth.llamarpc.com").await?;
    /// let manager = MulticallManager::new();
    /// // The deploy_multicall method is internal - use deploy_and_batch_call instead
    /// let calls = vec![/* your multicall data */];
    /// let _results = manager.deploy_and_batch_call(&mut evm, calls, false)?;
    /// # Ok(())
    /// # }
    /// ```
    fn deploy_multicall<DB, INSP>(&self, evm: &mut TraceEvm<DB, INSP>) -> Result<Address, EvmError>
    where
        DB: Database + DatabaseCommit,
    {
        // Deploy the Multicall contract using CREATE transaction
        let tx = TxEnv {
            kind: TxKind::Create,
            data: self.multicall_bytecode.clone(),
            chain_id: Some(evm.cfg.chain_id),
            ..Default::default()
        };

        // Execute the deployment transaction and commit to state
        let result = evm.transact_commit(tx).map_err(|e| {
            RuntimeError::ExecutionFailed(format!("Multicall deployment failed: {e}"))
        })?;

        // Process deployment result and extract contract address
        match result {
            ExecutionResult::Success { output, .. } => {
                match output.address() {
                    Some(address) => {
                        // Successfully deployed, return the contract address
                        Ok(*address)
                    }
                    None => {
                        // No address returned, deployment failed
                        Err(RuntimeError::Revert(
                            "Multicall deployment did not return a contract address".to_string(),
                        )
                        .into())
                    }
                }
            }
            ExecutionResult::Revert { output, .. } => Err(RuntimeError::Revert(format!(
                "Multicall deployment reverted: {}",
                String::from_utf8_lossy(&output)
            ))
            .into()),
            ExecutionResult::Halt { reason, .. } => {
                Err(RuntimeError::Revert(format!("Multicall deployment halted: {reason:?}")).into())
            }
        }
    }

    /// Deploy Multicall contract and execute batch calls in a single operation
    ///
    /// This is the main entry point for batch contract calls. It deploys a fresh
    /// Multicall contract and executes all provided calls through it, returning
    /// individual results for each call.
    ///
    /// # Arguments
    /// * `evm` - EVM instance for execution (must support database reset)
    /// * `calls` - Vector of MulticallCall structs defining target contracts and call data
    /// * `require_success` - Whether all calls must succeed (passed to tryAggregate)
    ///
    /// # Returns
    /// * `Ok(Vec<MulticallResult>)` - Results for each call, including success status and return data
    /// * `Err(EvmError)` - If deployment or batch execution fails
    ///
    /// # Implementation Details
    /// 1. Resets database to ensure clean state for deployment
    /// 2. Deploys Multicall contract using CREATE transaction
    /// 3. Encodes batch call data using tryAggregate function
    /// 4. Executes batch call transaction with appropriate nonce
    /// 5. Decodes and returns individual call results
    ///
    /// # Example
    /// ```no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use revm_trace::{create_evm, utils::multicall_utils::{MulticallManager, MulticallCall}};
    /// use alloy::primitives::{address, bytes};
    ///
    /// let mut evm = create_evm("https://eth.llamarpc.com").await?;
    /// let manager = MulticallManager::new();
    ///
    /// let calls = vec![
    ///     MulticallCall {
    ///         target: address!("A0b86a33E6417c9f36C6d2bAC7Cf26b6e7b2c3Ad"),
    ///         callData: bytes!("0x70a08231000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045"),
    ///     },
    /// ];
    ///
    /// let results = manager.deploy_and_batch_call(&mut evm, calls, false)?;
    /// for (i, result) in results.iter().enumerate() {
    ///     println!("Call {}: success={}, data={:?}", i, result.success, result.returnData);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn deploy_and_batch_call<DB, INSP>(
        &self,
        evm: &mut TraceEvm<CacheDB<DB>, INSP>,
        calls: Vec<MulticallCall>,
        require_success: bool,
    ) -> Result<Vec<MulticallResult>, EvmError>
    where
        DB: DatabaseRef,
    {
        // Handle empty calls case
        if calls.is_empty() {
            return Ok(Vec::new());
        }

        // Reset database to ensure clean state for deployment
        evm.reset_db();

        // Deploy Multicall contract and get its address
        let multicall_address = self.deploy_multicall(evm)?;

        // Encode the batch call data using tryAggregate function
        let multicall_data = tryAggregateCall {
            requireSuccess: require_success,
            calls,
        }
        .abi_encode();

        // Create transaction to call the deployed Multicall contract
        let tx = TxEnv {
            kind: TxKind::Call(multicall_address),
            data: multicall_data.into(),
            chain_id: Some(evm.cfg.chain_id),
            nonce: 1, // After deployment, nonce should start from 1
            ..Default::default()
        };

        // Execute the multicall transaction
        let execution_result = evm.transact(tx);

        match execution_result {
            Ok(execution_result) => {
                // Process the execution result
                match execution_result.result {
                    ExecutionResult::Success { output, .. } => {
                        // Decode the multicall results from the output data
                        let results: Vec<MulticallResult> = tryAggregateCall::abi_decode_returns(
                            &output.into_data(),
                        )
                        .map_err(|e| {
                            RuntimeError::DecodeError(format!(
                                "Failed to decode Multicall result: {e}"
                            ))
                        })?;

                        Ok(results)
                    }
                    ExecutionResult::Revert { output, .. } => Err(RuntimeError::Revert(format!(
                        "Multicall execution reverted: {}",
                        String::from_utf8_lossy(&output)
                    ))
                    .into()),
                    ExecutionResult::Halt { reason, .. } => Err(RuntimeError::Revert(format!(
                        "Multicall execution halted: {reason:?}",
                    ))
                    .into()),
                }
            }
            Err(e) => {
                // Handle execution error
                Err(
                    RuntimeError::ExecutionFailed(format!("Multicall execution failed: {e}"))
                        .into(),
                )
            }
        }
    }
}

impl Default for MulticallManager {
    /// Create a default MulticallManager instance
    ///
    /// This is equivalent to calling `MulticallManager::new()`.
    fn default() -> Self {
        Self::new()
    }
}
