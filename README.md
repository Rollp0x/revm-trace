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

- **Advanced Transaction Simulation**
  - Preview transaction outcomes without on-chain execution
  - Simulate complex DeFi interactions safely
  - Test multi-contract interactions
  - Support for all EVM-compatible chains

- **Comprehensive Analysis**
  - Track potential asset transfers (native and ERC20)
  - Analyze complete call traces
  - Identify state changes
  - Detect and locate errors
  - Collect and decode events

- **Developer-Friendly**
  - Safe, isolated simulation environment
  - Detailed execution reports
  - No gas fees or real transactions
  - Support for historical state analysis

## Installation

Add this to your `Cargo.toml`:
```toml
revm-trace = "2.0.0"
```

## Quick Start

```rust
use revm_trace::{
    create_evm,
    BlockEnv,
    SimulationTx,
    SimulationBatch,
    Tracer,
    types::TxKind,
    TransactionStatus,
};
use alloy::primitives::{address, U256};

async fn simulate_transfer_eth() -> anyhow::Result<()> {
    // Initialize simulation environment
    let mut evm = create_evm(
        "https://rpc.ankr.com/eth",
        Some(1), // Ethereum mainnet
        None,    // No custom configs
    )?;

    // Prepare transaction to simulate
    let tx = SimulationTx {
        caller: address!("dead00000000000000000000000000000000beef"),
        transact_to: TxKind::Call(address!("dac17f958d2ee523a2206206994597c13d831ec7")), // USDT
        value: U256::from(1000000000000000000u64), // 1 ETH
        data: vec![].into(), // Transaction data (e.g., swap function call)
    };

    // Simulate transaction and analyze potential outcomes
    let result = evm.trace_tx(
        tx,
        BlockEnv {
            number: 18000000,
            timestamp: 1700000000,
        },
    )?;

    // Analyze simulation results
    match result.execution_status() {
        TransactionStatus::Success => {
            println!("Transaction would succeed!");
            // Preview potential asset transfers
            for transfer in result.asset_transfers {
                println!(
                    "Predicted transfer: {} from {} to {}",
                    transfer.value, transfer.from, transfer.to
                );
            }
            // Preview emitted events
            for log in result.logs {
                println!("Expected event: {:?}", log);
            }
        }
        TransactionStatus::PartialSuccess => {
            println!("Transaction succeeded but with some internal errors");
        }
        TransactionStatus::Failed { error, origin_error } => {
            println!("Transaction would fail:");
            println!("Error: {}", error);
            if let Some(origin) = origin_error {
                println!("Original error: {}", origin);
            }
        }
    }

    Ok(())
}
```



## Usage Examples

### Simulating Multiple Transactions

```rust
let batch = SimulationBatch {
  block_env: BlockEnv {
    number: 18000000,
    timestamp: 1700000000,
  },
  is_multicall: true, // Simulate as atomic multicall
  transactions: vec![approve_tx, swap_tx, transfer_tx],
};
let results = evm.trace_txs(batch)?;
// Analyze combined effects of all transactions
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
let mut evm = create_evm("https://rpc...", Some(1), None)?;
let results: Vec<> = transactions
  .par_iter() // ❌ This will fail - EVM instance is not thread-safe
  .map(|tx| {
    evm.trace_tx(tx.clone(), block_env.clone()) // Sharing EVM across threads
  })
  .collect();
```

##### ✅ Correct Usage

1. **Sequential Processing**

```rust
// Process transactions sequentially with a single EVM instance
let mut evm = create_evm("https://rpc...", Some(1), None)?;
let results: Vec<> = transactions
  .iter()
  .map(|tx| evm.trace_tx(tx.clone(), block_env.clone()))
  .collect();
```

2. **Parallel Processing with Multiple Instances**

```rust
use rayon::prelude::;
// Create new EVM instance for each thread
let results: Vec<> = transactions
  .par_iter()
  .map(|tx| {
    // Each thread gets its own EVM instance
    let mut evm = create_evm("https://rpc...", Some(1), None)?;
    evm.trace_tx(tx.clone(), block_env.clone())
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

