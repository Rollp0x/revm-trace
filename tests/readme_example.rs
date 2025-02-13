use revm_trace::{
    TransactionProcessor,
    evm::create_evm_with_inspector,
    types::{BlockEnv, SimulationTx, SimulationBatch},
    inspectors::TxInspector,
};
use alloy::primitives::{address, U256, TxKind};

// Basic example showing core functionality
#[tokio::test(flavor = "multi_thread")]
async fn test_basic_usage() -> anyhow::Result<()> {
    // Initialize EVM with transaction inspector
    let mut evm = create_evm_with_inspector(
        "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
        TxInspector::new(),
    ).await?;

    // Create simulation transaction
    let tx = SimulationTx {
        caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
        transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
        value: U256::from(120000000000000000u64), //  0.12 ETH
        data: vec![].into(),
    };

    // Create batch with single transaction
    let batch = SimulationBatch {
        block_env: BlockEnv {
            number: 21784863,
            timestamp: 1700000000,
        },
        transactions: vec![tx],
        is_stateful: false,
    };

    // Execute transaction batch
    let results = evm.process_transactions(batch)
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
                        transfer.value, transfer.from, transfer.to.unwrap()
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
