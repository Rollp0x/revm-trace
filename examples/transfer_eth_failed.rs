
use alloy::primitives::{address, U256};
use anyhow::Result;
use colored::*;
use revm_trace::{create_evm_instance_with_inspector, trace_tx_assets, TransactionTracer};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize EVM with transaction tracer
    let inspector = TransactionTracer::default();
    let mut evm = create_evm_instance_with_inspector("https://rpc.ankr.com/eth", inspector, None)?;
    println!("{}", "✅ EVM instance created successfully\n".green());
    let safe = address!("Ab778bF14C7F879D33FAA7aeD44dA68AaA02513a");
    let to = address!("E8ccbb36816e5f2fB69fBe6fbd46d7e370435d84");
    let amount = U256::from(10000000000000000u128);
    let result = trace_tx_assets(&mut evm, safe, to, amount, vec![].into(), "ETH").await;
    assert!(result.error.is_some());
    assert_eq!(result.asset_transfers().len(), 0);
    println!("Error:❌ {}", result.error.unwrap().to_string().red());
    Ok(())
}
