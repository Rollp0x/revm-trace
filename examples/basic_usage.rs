//! ERC20 Token Transfer Simulation Example
//! 
//! This example demonstrates how to:
//! - Set up a simulation environment
//! - Encode ERC20 transfer calls
//! - Execute token transfers
//! - Track and verify transfer results
//! - Display transfer information with proper token details

use revm_trace::{
    TransactionTrace,create_evm_with_tracer,
    utils::erc20_utils::get_token_infos,
    SimulationBatch, SimulationTx, TxInspector
};
use anyhow::Result;
use alloy::{
    primitives::{address, utils::format_units, Address, U256,TxKind}, 
    sol, sol_types::SolCall
};

// Define ERC20 interface for transfer function
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


const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(
        ETH_RPC_URL,
        inspector,
    ).await?;

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

    let result = &evm.trace_transactions(SimulationBatch {
        block_env: None,
        is_stateful: false,
        transactions: vec![tx],
    }).into_iter().map(|v| v.unwrap()).collect::<Vec<_>>()[0];
    let output = result.0.output().unwrap();
    assert!(output.len() == 32 && output[31] == 1,"❌ Expected transfer to succeed");
    // Print results
    for transfer in &result.1.asset_transfers {
        let token_info = &get_token_infos(&mut evm, &[transfer.token], None).unwrap()[0];

        println!(
            "Transfer: {} {} -> {}: {}",
            token_info.symbol, transfer.from, transfer.to.unwrap(), format_units(transfer.value, token_info.decimals).unwrap()
        );
    }

    Ok(())
}

