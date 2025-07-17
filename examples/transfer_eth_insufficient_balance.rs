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

use alloy::primitives::{address, TxKind, U256};
use anyhow::Result;
use colored::*;
use revm_trace::{SimulationBatch, SimulationTx, TransactionTrace};

#[cfg(not(feature = "foundry-fork"))]
use revm_trace::create_evm;

#[cfg(feature = "foundry-fork")]
use revm_trace::create_shared_evm;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Starting transfer ETH failed test...");
    #[cfg(not(feature = "foundry-fork"))]
    println!("Using AlloyDB backend for EVM simulation");

    #[cfg(feature = "foundry-fork")]
    println!("Using Foundry fork backend for EVM simulation");

    #[cfg(not(feature = "foundry-fork"))]
    let mut evm = create_evm(ETH_RPC_URL).await?;

    #[cfg(feature = "foundry-fork")]
    let mut evm = create_shared_evm(ETH_RPC_URL).await?;

    println!("{}", "✅ EVM instance created successfully\n".green());

    // Configure transfer parameters
    let safe = address!("Ab778bF14C7F879D33FAA7aeD44dA68AaA02513a"); // Sender with insufficient balance
    let to = address!("E8ccbb36816e5f2fB69fBe6fbd46d7e370435d84"); // Recipient

    // Amount to transfer: 0.01 ETH
    let amount = U256::from(10000000000000000u128); // 0.01 ETH in wei

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
        transactions: vec![tx],
        is_stateful: true,
    };

    // Process transaction
    // Expected to fail due to insufficient balance
    let result = evm.trace_transactions(txs);

    // Verify failure
    assert!(
        result[0].is_err(),
        "❌ Expected transfer to fail due to insufficient balance"
    );

    // Display error details
    println!(
        "{}",
        format!("Result:❌ {:#?}", result[0].as_ref().unwrap_err()).red()
    );

    Ok(())
}
