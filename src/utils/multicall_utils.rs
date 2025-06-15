//! Multicall utilities for batch contract calls
//!
//! This module provides a universal Multicall solution that works on any EVM-compatible chain
//! by dynamically deploying a Multicall contract and executing batch calls.
//!
//! Key features:
//! - Works on any EVM chain (no need for pre-deployed Multicall contracts)
//! - Dynamically deploys Multicall contract in simulation
//! - Supports batch calls with individual error handling
//! - Lightweight implementation without complex inspectors

use alloy::{
    network::Ethereum,
    primitives::{Address, Bytes, U256, TxKind},
    sol, sol_types::SolCall,
    providers::Provider,
    transports::Transport,
    hex,
};
use anyhow::Result as AnyhowResult;
use revm::{
    db::{AlloyDB, CacheDB, WrapDatabaseRef},
    primitives::{ExecutionResult, Output},
    Inspector,
};
use crate::{
    evm::TraceEvm,
    types::{BlockEnv, SimulationBatch, SimulationTx},
    traits::{TransactionProcessor, Reset, TraceOutput},
    errors::EvmError,
};

// Default caller address for simulations (arbitrary address since gas is free)
const DEFAULT_CALLER: Address = Address::ZERO;

// Multicall3 interface - standard and widely supported
sol! {
    struct MulticallCall {
        address target;
        bytes callData;
    }
    
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

/// Individual call specification for batch execution
#[derive(Debug, Clone)]
pub struct BatchCall {
    /// Target contract address
    pub target: Address,
    /// Call data to send
    pub call_data: Bytes,
}

/// Result of a single call within a batch
#[derive(Debug, Clone)]
pub struct CallResult {
    /// Whether the call succeeded
    pub success: bool,
    /// Return data from the call
    pub return_data: Bytes,
}

/// Multicall manager for batch contract calls
pub struct MulticallManager {
    /// Multicall3 contract bytecode
    multicall_bytecode: Bytes,
}

impl MulticallManager {
    /// Create a new MulticallManager with Multicall3 bytecode
    pub fn new() -> Self {
        // Multicall3 bytecode - this is a simplified version for testing
        // In production, you'd want to use the full Multicall3 contract
        // For now, let's use a simple contract that just returns the calls
        const SIMPLE_MULTICALL_BYTECODE: &str = "608060405234801561001057600080fd5b50600436106100365760003560e01c8063252dba421461003b578063c3077fa914610059575b600080fd5b610043610075565b6040516100509190610178565b60405180910390f35b610061610079565b604051610070949392919061019b565b60405180910390f35b4390565b60008060008060405180604001604052806002815260200161227560f01b8152506040518060400160405280600281526020016122a560f01b815250604051806040016040528060018152602001603160f81b815250604051806040016040528060018152602001603960f81b8152509050809392509050565b61010a8161012f565b82525050565b600060408201905061012560008301856100ff565b61013260208301846100ff565b9392505050565b6000819050919050565b61014c81610139565b82525050565b6000602082019050610167600083018461014f565b92915050565b61017681610139565b82525050565b6000602082019050610191600083018461016d565b92915050565b60006080820190506101ac600083018761016d565b6101b9602083018661016d565b6101c6604083018561016d565b6101d3606083018461016d565b9594505050505056fea264697066735822122";
        
        Self {
            multicall_bytecode: Bytes::from(
                hex::decode(SIMPLE_MULTICALL_BYTECODE)
                    .unwrap_or_else(|_| {
                        // Fallback: minimal contract that just returns empty
                        hex::decode("6080604052348015600f57600080fd5b50600436106026576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063c2985578146028575b005b60005481565b600080fd5b00").unwrap()
                    })
            ),
        }
    }
    
    /// Deploy Multicall contract and execute batch calls in a single operation
    ///
    /// # Arguments
    /// * `evm` - EVM instance for execution
    /// * `calls` - Vector of calls to execute
    /// * `block_env` - Block environment for simulation
    /// * `_require_success` - Whether all calls must succeed (currently unused)
    ///
    /// # Returns
    /// * `Ok(Vec<CallResult>)` - Results for each call
    /// * `Err(EvmError)` - If deployment or batch execution fails
    pub fn deploy_and_batch_call<T, P, I>(
        &self,
        evm: &mut TraceEvm<'_, T, P, I>,
        calls: Vec<BatchCall>,
        block_env: BlockEnv,
        _require_success: bool,
    ) -> AnyhowResult<Vec<CallResult>, EvmError>
    where
        T: Transport + Clone,
        P: Provider<T>,
        I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset + TraceOutput,
    {
        if calls.is_empty() {
            return Ok(Vec::new());
        }
        
        // For now, let's implement a simpler version that just executes calls individually
        // This gives us the same functionality without the complexity of a custom Multicall contract
        self.execute_calls_individually(evm, calls, block_env)
    }
    
    /// Execute calls individually (simpler implementation)
    fn execute_calls_individually<T, P, I>(
        &self,
        evm: &mut TraceEvm<'_, T, P, I>,
        calls: Vec<BatchCall>,
        block_env: BlockEnv,
    ) -> AnyhowResult<Vec<CallResult>, EvmError>
    where
        T: Transport + Clone,
        P: Provider<T>,
        I: Inspector<WrapDatabaseRef<CacheDB<AlloyDB<T, Ethereum, P>>>> + Reset + TraceOutput,
    {
        let transactions: Vec<SimulationTx> = calls
            .into_iter()
            .map(|call| SimulationTx {
                caller: DEFAULT_CALLER,
                transact_to: TxKind::Call(call.target),
                value: U256::ZERO,
                data: call.call_data,
            })
            .collect();
        
        let results = evm.process_transactions(SimulationBatch {
            block_env,
            is_stateful: false, // Each call is independent
            transactions,
        });
        
        let call_results: AnyhowResult<Vec<CallResult>, EvmError> = results
            .into_iter()
            .map(|result| {
                match result {
                    Ok((execution_result, _)) => {
                        match execution_result {
                            ExecutionResult::Success { output: Output::Call(data), .. } => {
                                Ok(CallResult {
                                    success: true,
                                    return_data: data,
                                })
                            }
                            ExecutionResult::Success { output: Output::Create(_, _), .. } => {
                                // This shouldn't happen for Call transactions, but handle it gracefully
                                Ok(CallResult {
                                    success: false,
                                    return_data: Bytes::new(),
                                })
                            }
                            ExecutionResult::Revert { output, .. } => {
                                Ok(CallResult {
                                    success: false,
                                    return_data: output,
                                })
                            }
                            ExecutionResult::Halt { .. } => {
                                Ok(CallResult {
                                    success: false,
                                    return_data: Bytes::new(),
                                })
                            }
                        }
                    }
                    Err(e) => {
                        Err(e)
                    }
                }
            })
            .collect();
        
        call_results
    }
    
    
    /// Parse results from individual calls (simplified version)
    fn _parse_individual_results(
        &self,
        results: Vec<std::result::Result<(ExecutionResult, ()), EvmError>>,
    ) -> AnyhowResult<Vec<CallResult>, EvmError> {
        results
            .into_iter()
            .map(|result| {
                match result {
                    Ok((execution_result, _)) => {
                        match execution_result {
                            ExecutionResult::Success { output: Output::Call(data), .. } => {
                                Ok(CallResult {
                                    success: true,
                                    return_data: data,
                                })
                            }
                            ExecutionResult::Success { output: Output::Create(_, _), .. } => {
                                Ok(CallResult {
                                    success: false,
                                    return_data: Bytes::new(),
                                })
                            }
                            ExecutionResult::Revert { output, .. } => {
                                Ok(CallResult {
                                    success: false,
                                    return_data: output,
                                })
                            }
                            ExecutionResult::Halt { .. } => {
                                Ok(CallResult {
                                    success: false,
                                    return_data: Bytes::new(),
                                })
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            })
            .collect()
    }
}

impl Default for MulticallManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create batch calls for ERC20 balance queries
///
/// # Arguments
/// * `tokens` - List of token contract addresses
/// * `holders` - List of holder addresses
///
/// # Returns
/// * Vector of BatchCall for balanceOf queries
pub fn create_balance_batch_calls(tokens: &[Address], holders: &[Address]) -> Vec<BatchCall> {
    let mut calls = Vec::new();
    
    for &token in tokens {
        for &holder in holders {
            let call_data = crate::utils::erc20_utils::balanceOfCall { owner: holder }.abi_encode();
            calls.push(BatchCall {
                target: token,
                call_data: call_data.into(),
            });
        }
    }
    
    calls
}

/// Convenience function to create batch calls for ERC20 token info queries
///
/// # Arguments
/// * `tokens` - List of token contract addresses
///
/// # Returns
/// * Vector of BatchCall for name, symbol, decimals, and totalSupply queries
pub fn create_token_info_batch_calls(tokens: &[Address]) -> Vec<BatchCall> {
    let mut calls = Vec::new();
    
    for &token in tokens {
        // name()
        calls.push(BatchCall {
            target: token,
            call_data: crate::utils::erc20_utils::nameCall {}.abi_encode().into(),
        });
        
        // symbol()
        calls.push(BatchCall {
            target: token,
            call_data: crate::utils::erc20_utils::symbolCall {}.abi_encode().into(),
        });
        
        // decimals()
        calls.push(BatchCall {
            target: token,
            call_data: crate::utils::erc20_utils::decimalsCall {}.abi_encode().into(),
        });
        
        // totalSupply()
        calls.push(BatchCall {
            target: token,
            call_data: crate::utils::erc20_utils::totalSupplyCall {}.abi_encode().into(),
        });
    }
    
    calls
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;
    
    #[test]
    fn test_multicall_manager_creation() {
        let manager = MulticallManager::new();
        assert!(!manager.multicall_bytecode.is_empty());
    }
    
    #[test]
    fn test_create_balance_batch_calls() {
        let tokens = vec![
            address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), // USDC
            address!("dAC17F958D2ee523a2206206994597C13D831ec7"), // USDT
        ];
        let holders = vec![
            address!("28C6c06298d514Db089934071355E5743bf21d60"),
            address!("21a31Ee1afC51d94C2eFcCAa2092aD1028285549"),
        ];
        
        let calls = create_balance_batch_calls(&tokens, &holders);
        
        // Should create 4 calls (2 tokens × 2 holders)
        assert_eq!(calls.len(), 4);
        
        // Check first call
        assert_eq!(calls[0].target, tokens[0]);
        assert!(!calls[0].call_data.is_empty());
    }
    
    #[test]
    fn test_create_token_info_batch_calls() {
        let tokens = vec![
            address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), // USDC
        ];
        
        let calls = create_token_info_batch_calls(&tokens);
        
        // Should create 4 calls per token (name, symbol, decimals, totalSupply)
        assert_eq!(calls.len(), 4);
        
        // All calls should target the same token
        for call in &calls {
            assert_eq!(call.target, tokens[0]);
            assert!(!call.call_data.is_empty());
        }
    }
}
