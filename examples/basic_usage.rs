use alloy::{
    eips::BlockNumberOrTag, 
    primitives::{address, Address, U256,utils::format_units}, 
    providers::{Provider, ProviderBuilder}, 
    sol, sol_types::SolCall
};
use anyhow::Result;
use revm_trace::{create_evm,Tracer,SimulationTx,types::TxKind,BlockEnv,SimulationBatch};
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
fn encode_erc20_transfer(to: Address, amount: U256) -> Vec<u8> {
    ERC20::transferCall { to, amount }.abi_encode()
}

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
    let mut evm = create_evm("https://rpc.ankr.com/eth",Some(1),None)?;

    // USDC proxy contract address
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");

    // Construct transfer call data
    let transfer_data = encode_erc20_transfer(
        address!("34e5dacdc16ff5bcdbdfa66c21a20f46347d86cf "),
        U256::from(1000000), // 1 USDC (6 decimals)
    );
    let tx = SimulationTx {
        caller: address!("28C6c06298d514Db089934071355E5743bf21d60"),
        transact_to: TxKind::Call(usdc),
        value: U256::ZERO,
        data: transfer_data.into(),
    };
    let block_env = get_block_env("https://rpc.ankr.com/eth",None).await;

    let result = evm.trace_txs(SimulationBatch {
        block_env,
        transactions: vec![tx],
        is_bound_multicall: false,
    }).unwrap()[0].clone();
    println!("{:#?}",result);
    
    // Print results
    for transfer in result.asset_transfers {
        let token_info = result
            .token_infos
            .get(&transfer.token)
            .expect("Token info should exist");
        println!(
            "Transfer: {} {} -> {}: {}",
            token_info.symbol, transfer.from, transfer.to.unwrap(), format_units(transfer.value, token_info.decimals).unwrap()
        );
    }

    Ok(())
}
