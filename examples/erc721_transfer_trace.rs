use alloy::{
    primitives::{address, TxKind, U256},
    sol,
    sol_types::SolCall,
};
use anyhow::Result;
use revm_trace::{
    create_evm_with_tracer, SimulationBatch, SimulationTx, TransactionTrace, TxInspector,
};

sol! {
    contract BoredApeYachtClub {
        function safeTransferFrom(address from, address to, uint256 tokenId) external;
    }
}

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer(ETH_RPC_URL, inspector).await?;
    let caller = address!("0x7eb413211a9DE1cd2FE8b8Bb6055636c43F7d206");
    let receiver = address!("0x28C6c06298d514Db089934071355E5743bf21d60");
    let bayc = address!("0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D");
    let token_id = U256::from(811);
    let data = BoredApeYachtClub::safeTransferFromCall {
        from: caller,
        to: receiver,
        tokenId: token_id,
    }
    .abi_encode();
    let tx = SimulationTx {
        caller,
        transact_to: TxKind::Call(bayc),
        value: U256::ZERO,
        data: data.into(),
    };
    let result = evm
        .trace_transactions(SimulationBatch {
            transactions: vec![tx],
            is_stateful: true,
        })
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>()[0]
        .clone();

    println!("\nTransaction Result:");
    println!("-----------------");
    println!("State diff: {:?}", result.1);
    println!("Call Trace: {:?}", result.2.call_trace.unwrap());
    assert!(result.0.is_success(), "❌ Transfer failed");
    assert!(result.2.asset_transfers.len() == 1, "❌ No transfers found");
    for transfer in &result.2.asset_transfers {
        println!(
            "Token: {} | Transfer: {} -> {:?} | Type: {:?}, TokenID: {:?}, Amount: {}",
            transfer.token,
            transfer.from,
            transfer.to,
            transfer.token_type,
            transfer.id,
            transfer.value
        );
    }
    Ok(())
}
