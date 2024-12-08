//! ETH Transfer Validation Failure Example
//! 
//! This example shows how EVM handles transactions that fail during the validation phase,
//! specifically when attempting to transfer ETH with insufficient balance.
//! 
//! Key points demonstrated:
//! - Transaction validation occurs before execution
//! - No gas is consumed when validation fails
//! - No execution trace is generated
//! - How to handle and verify expected failures
//! 
//! This example demonstrates:
//! - How EVM handles pre-execution validation failures
//! - Transfer attempt with insufficient balance
//! - Error handling in transaction simulation
//! 
//! Note: This transaction fails during validation phase (before execution),
//! so there will be no execution trace or inspector output.

use revm_trace::{
    types::TxKind,
    create_evm_with_inspector, SimulationBatch, SimulationTx, TxInspector
};
use anyhow::Result;
use alloy::primitives::{address, U256};
use colored::*;
mod common;
use common::get_block_env;

const ETH_RPC_URL: &str = "https://rpc.ankr.com/eth";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting transfer ETH failed test...");
    
    // Initialize EVM with transaction inspector
    // Note: Inspector won't produce output as transaction fails in validation
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_inspector(ETH_RPC_URL, inspector).await.unwrap();
    
    // Get latest block environment for simulation
    let block_env = get_block_env(ETH_RPC_URL, None).await.unwrap();
    println!("{}", "✅ EVM instance created successfully\n".green());

    // Configure transfer parameters
    let safe = address!("Ab778bF14C7F879D33FAA7aeD44dA68AaA02513a");  // Sender with insufficient balance
    let to = address!("E8ccbb36816e5f2fB69fBe6fbd46d7e370435d84");    // Recipient
    
    // Amount to transfer: 0.01 ETH
    let amount = U256::from(10000000000000000u128);  // 0.01 ETH in wei

    // Create transfer transaction
    // Empty data field as this is a simple ETH transfer
    let tx = SimulationTx {
        caller: safe,
        transact_to: TxKind::Call(to),
        value: amount,
        data: vec![].into(),
    };

    // Create transaction batch
    // Note: is_stateful doesn't matter here as transaction will fail validation
    let txs = SimulationBatch {
        block_env,
        transactions: vec![tx],
        is_stateful: true,
    };

    // Process transaction
    // Expected to fail due to insufficient balance
    let result = evm.process_transactions(txs);

    // Verify failure
    assert!(
        result.is_err(),
        "❌ Expected transfer to fail due to insufficient balance"
    );

    // Display error details
    println!(
        "{}",
        format!("Result:❌ {:#?}", result.unwrap_err()).red()
    );

    Ok(())
}