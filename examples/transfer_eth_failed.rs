
use alloy::{
    providers::{Provider, ProviderBuilder},
    eips::BlockNumberOrTag,
    primitives::{address, U256}
};
use anyhow::Result;
use colored::*;
use revm_trace::{create_evm ,SimulationTx,Tracer,types::TxKind,BlockEnv};

async fn get_block_env(http_url: &str,block_number:Option<u64>) -> BlockEnv {
    let provider = ProviderBuilder::new()
        .on_http(http_url.parse().unwrap());
    if let Some(block_number) = block_number {
        let block_info = provider.get_block_by_number(BlockNumberOrTag::Number(block_number),false).await.unwrap().unwrap();
        return BlockEnv { number: block_number, timestamp: block_info.header.timestamp };
    } else {
        let latest_block = provider.get_block_number().await.unwrap();
        let block_info = provider.get_block_by_number(BlockNumberOrTag::Number(latest_block),false).await.unwrap().unwrap();
        return BlockEnv { number: latest_block, timestamp: block_info.header.timestamp };
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize EVM with transaction tracer
    let mut evm = create_evm("https://rpc.ankr.com/eth",Some(1),None)?;
    println!("{}", "✅ EVM instance created successfully\n".green());
    let safe = address!("Ab778bF14C7F879D33FAA7aeD44dA68AaA02513a");
    let to = address!("E8ccbb36816e5f2fB69fBe6fbd46d7e370435d84");
    let amount = U256::from(10000000000000000u128);
    let block_env = get_block_env("https://rpc.ankr.com/eth",None).await;
    let tx = SimulationTx{
        caller: safe,
        transact_to: TxKind::Call(to),
        value: amount,
        data: vec![].into(),
    };
    let result = evm.trace_tx(tx, block_env).unwrap();
    assert!(!result.is_success(),"❌ Expected transfer to fail");
    assert!(result.asset_transfers.is_empty(),"❌ Expected no transfers");
    println!("{}",format!("Status:❌ {:#?}", result.status).red());
    Ok(())
}
