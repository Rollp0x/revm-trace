// examples/common/mod.rs
use revm_trace::BlockEnv;
use alloy::{
    eips::BlockNumberOrTag,
    providers::{ProviderBuilder,Provider},
};
use anyhow::Result;

/// Get block environment for simulation
/// 
/// # Arguments
/// * `http_url` - RPC endpoint URL
/// * `block_number` - Optional block number, uses latest if None
/// 
/// # Returns
/// BlockEnv containing block number and timestamp
pub async fn get_block_env(http_url: &str, block_number: Option<u64>) -> Result<BlockEnv> {
    let provider = ProviderBuilder::new()
        .on_http(http_url.parse()?);
    
    if let Some(block_number) = block_number {
        let block_info = provider
            .get_block_by_number(BlockNumberOrTag::Number(block_number), false)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
        Ok(BlockEnv { 
            number: block_number, 
            timestamp: block_info.header.timestamp 
        })
    } else {
        let latest_block = provider.get_block_number().await?;
        let block_info = provider
            .get_block_by_number(BlockNumberOrTag::Number(latest_block), false)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
        Ok(BlockEnv { 
            number: latest_block, 
            timestamp: block_info.header.timestamp 
        })
    }
}