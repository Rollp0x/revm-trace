# REVM Transaction Simulator and Analyzer

A Rust library that combines powerful transaction simulation with comprehensive analysis capabilities for EVM-based blockchains. Built on [REVM](https://github.com/bluealloy/revm), this tool enables you to:

- **Simulate** complex transactions and their interactions before actual execution
- **Analyze** potential outcomes, asset transfers, and state changes
- **Detect** possible errors and their root causes
- **Preview** all transaction effects in a safe, isolated environment

Perfect for:
- DeFi developers testing complex interactions
- Wallet developers validating transaction safety
- Protocol teams analyzing contract behaviors
- Security researchers investigating transaction patterns

## Key Features

- **Flexible Inspector System**
  - Built on REVM's inspector framework
  - Custom `TxInspector` for transaction analysis
  - Support for custom inspector implementations
  - Comprehensive asset transfer tracking

- **Complete Call Hierarchy Analysis**
  - Full depth call stack tracking
  - Detailed call context information
  - Internal transaction tracing
  - Precise error location in call stack
  - Step-by-step execution tracing

- **Enhanced Error Handling**
  - Detailed error messages and traces
  - Error location in call stack
  - Revert reason decoding
  - Custom error parsing
  - Contract-specific error context

- **Batch Transaction Processing**
  - Process multiple transactions
  - Stateful/stateless execution modes
  - Automatic state management
  - Detailed execution results

- **Asset Analysis**
  - Native token transfers
  - ERC20 token transfers
  - Transfer event parsing
  - Balance change tracking
  - Complete transaction logs

## Features
  - `async` - Enable async support
  - `ws` - WebSocket provider support
  - `http` - HTTP provider support (default)


## Installation

Add this to your `Cargo.toml`:
```toml
revm-trace = "2.0.1"
```

## Quick Start

```rust
use revm_trace::{
    TransactionProcessor,
    evm::create_evm_with_inspector,
    types::{BlockEnv, SimulationTx, SimulationBatch},
    inspectors::TxInspector,
};
use alloy::primitives::{address, U256, TxKind};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
```



## More Examples

For more detailed examples and use cases, please check:

- [Example Directory](./examples/): Contains standalone examples demonstrating specific features
  - DeFi interaction simulations
  - Token transfer analysis
  - Complex contract interactions
  - Proxy contract handling

- [Integration Tests](./tests/trace_tests.rs): Comprehensive test cases showing various usage scenarios
  - Transaction batching
  - Error handling
  - State tracking
  - Event analysis

These examples cover common use cases and demonstrate best practices for using the library.

For a quick overview, here are some key examples:

1. [Simulating DeFi Swaps](./examples/defi_swap.rs)
2. [Analyzing Token Transfers](./examples/token_transfer.rs)
3. [Handling Complex Contract Interactions](./examples/contract_interaction.rs)
4. [Working with Proxy Contracts](./examples/proxy_contracts.rs)



## Important Notes

### Safe Simulation Environment

All simulations run in an isolated environment:
- No actual blockchain state is modified
- No real transactions are submitted
- No gas fees are spent
- Perfect for testing and validation

### Thread Safety and Concurrency

The EVM instance is not thread-safe and cannot be shared between threads. Here's how to handle concurrent operations:

##### ❌ What NOT to do

```rust
// DON'T share a single EVM instance across threads
let mut evm = create_evm_with_inspector("https://rpc...", TxInspector::new()).await?;
let results: Vec<_> = transactions
  .par_iter() // ❌ This will fail - EVM instance is not thread-safe
  .map(|tx| {
    evm.process_transactions(SimulationBatch {
      block_env: block_env.clone(),
      transactions: vec![tx.clone()],
      is_stateful: true,
    }) // Sharing EVM across threads
  })
  .collect();
```

##### ✅ Correct Usage

1. **Sequential Processing**

```rust
// Process transactions sequentially with a single EVM instance
let mut evm = create_evm_with_inspector("https://rpc...", TxInspector::new()).await?;
let results: Vec<_> = transactions
  .iter()
  .map(|tx| {
    evm.process_transactions(SimulationBatch {
        block_env: block_env.clone(),
        transactions: vec![tx.clone()],
        is_stateful: true,
      })
    })
  .collect();
```

2. **Parallel Processing with Multiple Instances**

```rust
use rayon::prelude::*;
// Create new EVM instance for each thread
let results: Vec<Result<_, _>> = transactions
  .par_iter()
  .map(|tx| async {
    // Each thread gets its own EVM instance
    let mut evm = create_evm_with_inspector("https://rpc...", TxInspector::new()).await?;
    evm.process_transactions(SimulationBatch {
      block_env: block_env.clone(),
      transactions: vec![tx.clone()],
      is_stateful: true,
    })
  })
  .collect();
```



#### Performance Considerations

- **RPC Limitations**: 

  - Each EVM instance maintains its own RPC connection
  - Consider your RPC provider's connection and rate limits
  - Too many parallel instances might exceed provider limits

- **Resource Usage**:
  - Each EVM instance requires its own memory
  - Balance parallelism with resource constraints
  - Monitor system memory usage when scaling

- **Optimal Approach**:
  - For small batches: Use sequential processing
  
  - For large batches: Use parallel processing with connection pooling
  
  - Consider implementing a worker pool pattern for better resource management
  
    

### Working with Proxy Contracts

The library automatically handles proxy contracts by resolving their implementations:
- EIP-1967 proxies
- EIP-1967 beacon proxies
- OpenZeppelin transparent proxies
- EIP-1822 (UUPS) proxies

## Features in Detail

### Asset Transfer Tracking
- Native token transfers (including internal transfers)
- ERC20 token transfers
- Transaction logs and events
- Chronological ordering of transfers
- Complete token information collection

### Transaction Simulation
- Full EVM context simulation
- Custom environment configuration
- Detailed execution results
- Error handling and revert messages

## Historical State Access

Simulations can be run against different historical states:
- Recent blocks: Available on all nodes
- Historical blocks: Requires archive node access
- Future blocks: Uses latest state as base

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Built with [REVM](https://github.com/bluealloy/revm) and [Alloy](https://github.com/alloy-rs/alloy)

