use alloy::{
    primitives::{address, Address, U256},
    sol,
    sol_types::SolCall,
};
use anyhow::Result;
use revm_trace::{create_evm_instance_with_inspector, trace_tx_assets, TransactionTracer};

sol!(
    contract ERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
);

/// Encodes an ERC20 transfer function call
///
/// # Arguments
/// * `to` - The recipient address
/// * `amount` - The amount to transfer
///
/// # Returns
/// The encoded function call as bytes
pub fn encode_erc20_transfer(to: Address, amount: U256) -> Vec<u8> {
    ERC20::transferCall { to, amount }.abi_encode()
}

#[tokio::main]
async fn main() -> Result<()> {
    let inspector = TransactionTracer::default();
    let mut evm = create_evm_instance_with_inspector("https://rpc.ankr.com/eth", inspector, None)?;

    // USDC 代理合约地址
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    // 构造转账调用数据
    let transfer_data = encode_erc20_transfer(
        address!("34e5dacdc16ff5bcdbdfa66c21a20f46347d86cf "),
        U256::from(1000000), // 1 USDC (6 decimals)
    );

    let result = trace_tx_assets(
        &mut evm,
        address!("28C6c06298d514Db089934071355E5743bf21d60"),
        usdc,
        U256::ZERO,
        transfer_data,
        "ETH",
    )
    .await;
    // 打印结果
    for transfer in result.asset_transfers() {
        let token_info = result
            .token_info
            .get(&transfer.token)
            .expect("Token info should exist");
        println!(
            "Transfer: {} {} -> {}: {}",
            token_info.symbol, transfer.from, transfer.to, transfer.value
        );
    }

    Ok(())
}
