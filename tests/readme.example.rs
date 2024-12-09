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
        caller: address!("dead00000000000000000000000000000000beef"),
        transact_to: TxKind::Call(address!("dac17f958d2ee523a2206206994597c13d831ec7")),
        value: U256::from(1000000000000000000u64), // 1 ETH
        data: vec![].into(),
    };

    // Create batch with single transaction
    let batch = SimulationBatch {
        block_env: BlockEnv {
            number: 18000000,
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
        match execution_result.is_success {
            true => {
                println!("Transaction succeeded!");
                for transfer in inspector_output.asset_transfers {
                    println!(
                        "Transfer: {} from {} to {}",
                        transfer.value, transfer.from, transfer.to
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

// Advanced example showing batch processing and state control
#[tokio::test(flavor = "multi_thread")]
async fn test_advanced_usage() -> anyhow::Result<()> {
    // Initialize with custom inspector configuration
    let inspector = TxInspector::new()
        .with_transfer_tracking(true)
        .with_error_tracking(true);
    
    let mut evm = create_evm_with_inspector(
        "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
        inspector,
    ).await?;

    // Create multiple transactions
    let tx1 = SimulationTx {
        caller: address!("dead00000000000000000000000000000000beef"),
        transact_to: TxKind::Call(address!("dac17f958d2ee523a2206206994597c13d831ec7")),
        value: U256::from(1000000000000000000u64),
        data: vec![].into(),
    };

    let tx2 = SimulationTx {
        caller: address!("dead00000000000000000000000000000000beef"),
        transact_to: TxKind::Call(address!("dac17f958d2ee523a2206206994597c13d831ec7")),
        value: U256::from(2000000000000000000u64),
        data: vec![].into(),
    };

    // Process batch with state preservation
    let batch = SimulationBatch {
        block_env: BlockEnv {
            number: 18000000,
            timestamp: 1700000000,
        },
        transactions: vec![tx1, tx2],
        is_stateful: true, // Maintain state between transactions
    };

    let results = evm.process_transactions(batch);
    
    // Process results...
    
    Ok(())
}