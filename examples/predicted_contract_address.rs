//! Test getting contract address from ExecutionResult
//! 
//! This example demonstrates how to get deployed contract address
//! directly from ExecutionResult without using inspectors.

use revm_trace::{
    SimulationBatch, SimulationTx,
};
use anyhow::Result;
use revm::database::Database;
use revm::context::ContextTr;
use alloy::{
    primitives::{address, hex, Address, U256,TxKind}, 
    sol
};
use revm::context_interface::result::ExecutionResult;

#[cfg(not(feature = "foundry-fork"))]
use revm_trace::create_evm;

#[cfg(feature = "foundry-fork")]
use revm_trace::create_shared_evm;

// Define a simple contract
sol! {
    contract OwnerDemo {
        address private _owner;
        constructor() {
            _owner = msg.sender;
        }
        function getOwner() public view returns (address) {
            return _owner;
        }
    }
}

const BYTECODE: &str = "608060405234801561001057600080fd5b50600080546001600160a01b031916331790556101ca806100326000396000f3fe608060405234801561001057600080fd5b506004361061002b5760003560e01c8063893d20e814610030575b600080fd5b61003861004e565b6040516100459190610146565b60405180910390f35b60008054906101000a90046001600160a01b031690565b600073ffffffffffffffffffffffffffffffffffffffff82169050919050565b600061009182610066565b9050919050565b6100a181610086565b82525050565b60006020820190506100bc6000830184610098565b92915050565b600080fd5b6100d081610086565b81146100db57600080fd5b50565b6000813590506100ed816100c7565b92915050565b600060208284031215610109576101086100c2565b5b6000610117848285016100de565b91505092915050565b600061012b82610066565b9050919050565b61013b81610120565b82525050565b60006020820190506101566000830184610132565b9291505056fea2646970667358221220892a63a629c1a45fb9de27ff5b76c3bb4c5ff8e7df8b5c6b4d6e4c3f1c06b42564736f6c63430008110033";
const SENDER: Address = address!("b20a608c624Ca5003905aA834De7156C68b2E1d0");
const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Testing contract address extraction from ExecutionResult...");
    
    #[cfg(not(feature = "foundry-fork"))]
    println!("Using AlloyDB backend for EVM simulation");
    
    #[cfg(feature = "foundry-fork")]
    println!("Using Foundry fork backend for EVM simulation");
    // Create basic EVM instance without inspector
    #[cfg(not(feature = "foundry-fork"))]
    let mut evm = create_evm(ETH_RPC_URL).await?;

    #[cfg(feature = "foundry-fork")]
    let mut evm = create_shared_evm(ETH_RPC_URL).await?;

    // Predict contract address
    let current_account = evm.db().basic(SENDER).unwrap().unwrap();
    let nonce = current_account.nonce;
    let predicted_address = SENDER.create(nonce);
    println!("Predicted contract address: {}", predicted_address);
    
    // Deploy contract
    let deploy_tx = SimulationTx {
        caller: SENDER,
        transact_to: TxKind::Create,
        value: U256::ZERO,
        data: hex::decode(BYTECODE).unwrap().into(),
    };
    
    // Execute deployment
    let results = evm.execute_batch(SimulationBatch {
        is_stateful: false,
        transactions: vec![deploy_tx],
    });
    
    // Check the result
    match &results[0] {
        Ok(execution_result) => {
            println!("Deployment successful!");
            
            // Try to extract contract address from ExecutionResult
            match execution_result {
                ExecutionResult::Success { output, .. } => {
                    match output.address() {
                        Some(address) => {
                            println!("✅ Contract deployed at address: {}", address);
                            assert_eq!(*address, predicted_address, "❌ Contract address does not match predicted address");
                        }
                        None=> {
                            println!("❌ Created contract address is None, expected address: {}", predicted_address);
                        }
                    }
                }
                _ => {
                    println!("❌ Deployment failed: {:?}", execution_result);
                }
            }
        }
        Err(e) => {
            println!("❌ Transaction failed: {:?}", e);
        }
    }
    
    Ok(())
}
