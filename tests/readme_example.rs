use alloy::primitives::{address, TxKind, U256};
use revm_trace::{
    types::{SimulationBatch, SimulationTx},
    EvmBuilder, TransactionTrace, TxInspector,
};

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

// Basic example showing core functionality
#[tokio::test(flavor = "multi_thread")]
async fn test_basic_usage() -> anyhow::Result<()> {
    // Initialize EVM with transaction inspector
    let inspector = TxInspector::new();
    let mut evm = EvmBuilder::new_alloy(ETH_RPC_URL)
        .with_block_number(21784863)
        .with_tracer(inspector)
        .build()
        .await?;

    let sender = address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61");
    // Create simulation transaction
    let tx = SimulationTx {
        caller: sender,
        transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
        value: U256::from(120000000000000000u64), //  0.12 ETH
        data: vec![].into(),
    };

    // Create batch with single transaction
    let batch = SimulationBatch {
        transactions: vec![tx],
        is_stateful: false,
    };

    // Execute transaction batch
    let results = evm
        .trace_transactions(batch)
        .into_iter()
        .map(|v| v.unwrap())
        .collect::<Vec<_>>();

    // Process results
    for (execution_result, inspector_output) in results {
        match execution_result.is_success() {
            true => {
                println!("Transaction succeeded!");
                for transfer in inspector_output.asset_transfers {
                    println!(
                        "Transfer: {} from {} to {}",
                        transfer.value,
                        transfer.from,
                        transfer.to.unwrap()
                    );
                }
            }
            false => {
                println!("Transaction failed!");
                if let Some(error_trace) = inspector_output.error_trace_address {
                    println!("Error occurred at call depth: {}", error_trace.len());
                }
            }
        }
    }

    Ok(())
}
