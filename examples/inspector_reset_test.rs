use revm_trace::{create_evm_with_tracer,TransactionTrace, TxInspector, SimulationBatch, SimulationTx};
use alloy::primitives::{address, U256, TxKind};

/// Test to verify that inspector state is properly reset between transactions in batch processing
/// 
/// This test creates a SINGLE BATCH containing two ETH transfers and verifies that:
/// 1. Both transactions are processed in the same batch call
/// 2. Inspector state is properly reset between transactions within the batch
/// 3. Simple check: if asset_transfers lengths are equal but contents differ, reset worked
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Inspector Reset in Single Batch Processing");
    println!("{}", "=".repeat(60));

    // Create EVM with tracer
    let trace_inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", trace_inspector).await?;

    // Define test addresses
    let from = address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"); // Vitalik's address
    let to1 = address!("0000000000000000000000000000000000000001"); // First recipient
    let to2 = address!("0000000000000000000000000000000000000002"); // Second recipient
    let value = U256::from(1000000000000000u64); // 0.001 ETH

    // Create single batch containing both transactions using SimulationTx
    let tx1 = SimulationTx {
        caller: from,
        transact_to: TxKind::Call(to1),
        value,
        data: vec![].into(),
    };

    let tx2 = SimulationTx {
        caller: from,
        transact_to: TxKind::Call(to2),
        value,
        data: vec![].into(),
    };

    let single_batch = SimulationBatch {
        block_env: None,
        transactions: vec![tx1, tx2],
        is_stateful: false, // stateless mode
    };

    println!("📦 Processing SINGLE BATCH containing {} transactions:", single_batch.transactions.len());
    println!("   📤 TX1: {} -> {} (value: {})", from, to1, value);
    println!("   📤 TX2: {} -> {} (value: {})", from, to2, value);
    println!("   🎯 Key Test: Inspector should reset between TX1 and TX2 within this batch");

    // Execute the single batch containing both transactions
    let results: Vec<_> = evm.trace_transactions(single_batch)
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    println!("\n✅ Single batch execution completed with {} results", results.len());

    // Extract outputs from each transaction
    if results.len() != 2 {
        return Err(format!("Expected 2 results, got {}", results.len()).into());
    }

    let (_result1, output1) = &results[0];
    let (_result2, output2) = &results[1];

    println!("\n📊 Transaction 1 Results:");
    println!("   💰 Asset transfers: {}", output1.asset_transfers.len());
    if let Some(transfer) = output1.asset_transfers.first() {
        println!("   🎯 First transfer to: {:?}", transfer.to);
    }

    println!("\n📊 Transaction 2 Results:");
    println!("   💰 Asset transfers: {}", output2.asset_transfers.len());
    if let Some(transfer) = output2.asset_transfers.first() {
        println!("   🎯 First transfer to: {:?}", transfer.to);
    }

    // Simplified verification: lengths equal but contents different = reset worked
    println!("\n🔍 Verification Results:");
    println!("{}", "=".repeat(60));

    if output1.asset_transfers.len() == output2.asset_transfers.len() {
        println!("✅ PASS: Asset transfer lengths are equal ({} each)", output1.asset_transfers.len());
        
        // Check if contents are different (which proves reset worked)
        let contents_different = if let (Some(transfer1), Some(transfer2)) = (
            output1.asset_transfers.first(),
            output2.asset_transfers.first()
        ) {
            transfer1.to != transfer2.to
        } else {
            false
        };

        if contents_different {
            println!("✅ PASS: Asset transfer contents are different");
            println!("   🎉 Inspector was properly reset between transactions!");
            println!("   ✨ TX1 and TX2 have independent outputs despite being in same batch");
        } else {
            println!("❌ FAIL: Asset transfer contents are the same");
            println!("   🚨 Inspector was NOT reset between transactions!");
            return Err("Inspector reset test failed - contents identical".into());
        }
    } else {
        println!("❌ FAIL: Asset transfer lengths differ ({} vs {})", 
                 output1.asset_transfers.len(), output2.asset_transfers.len());
        println!("   🚨 This suggests state accumulation or other issues");
        return Err("Inspector reset test failed - length mismatch".into());
    }

    println!("\n🎉 All tests PASSED! Inspector reset is working correctly within single batch!");
    println!("   ✨ Both transactions were processed in the same batch call");
    println!("   ✨ Equal lengths + different contents = proper reset");
    println!("   ✨ No state accumulation between TX1 and TX2 within the batch");

    Ok(())
}
