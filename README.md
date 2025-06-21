# REVM Transaction Simulator and Analyzer v3.0

A high-performance, **multi-threaded** Rust library that combines powerful transaction simulation with comprehensive analysis capabilities for EVM-based blockchains. Built on [REVM](https://github.com/bluealloy/revm), this tool enables you to:

- **Simulate** complex transactions and their interactions before actual execution
- **Analyze** potential outcomes, asset transfers, and state changes  
- **Detect** possible errors and their root causes
- **Preview** all transaction effects in a safe, isolated environment
- **Process** multiple transactions concurrently with built-in thread safety

Perfect for:
- DeFi developers testing complex interactions
- Wallet developers validating transaction safety
- Protocol teams analyzing contract behaviors
- Security researchers investigating transaction patterns
- High-throughput applications requiring concurrent transaction processing

## 🚀 What's New in v3.0

- **🔥 Multi-Threading by Default**: All EVM instances are now thread-safe and optimized for concurrent processing
- **⚡ Dual EVM Modes**: Choose between high-performance execution or detailed tracing based on your needs
- **🎯 Simplified API**: Unified interface with `create_evm()` and `create_evm_with_tracer()` functions
- **🌐 Universal Protocol Support**: Seamless HTTP/WebSocket support with automatic connection management

## Key Features

- **Dual EVM Mode Support**
  - **Standard EVM**: Ultra-fast execution without tracing (`create_evm()`)
  - **Tracing EVM**: Full transaction analysis with comprehensive trace data (`create_evm_with_tracer()`)
  - Seamless switching between modes based on your requirements
  - Built-in thread safety for concurrent processing

- **Multi-Threading by Default**
  - All EVM instances are thread-safe out of the box
  - Shared cache database for optimal performance
  - Concurrent transaction simulation and analysis
  - Optimized for high-throughput applications

- **Flexible Inspector System**
  - Built on REVM's inspector framework
  - Custom `TxInspector` for detailed transaction analysis
  - Support for custom inspector implementations
  - Comprehensive asset transfer tracking
  - Optional no-op inspector for performance-critical scenarios

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

- **Universal Multicall Support**
  - Dynamic Multicall contract deployment
  - Batch execution of multiple contract calls
  - Works on any EVM-compatible chain
  - Zero dependency on pre-deployed contracts
  - Optimized for cross-chain compatibility
  - Support for 100+ calls in single batch

## Features
  - `async` - Enable async support
  - `ws` - WebSocket provider support
  - `http` - HTTP provider support (default)

### TLS Implementation Options

By default, this library uses the system's native TLS implementation (typically OpenSSL). However, you can switch to a pure Rust TLS implementation:

- **rustls-tls**: Uses rustls instead of native-tls (OpenSSL)

```toml
# In your Cargo.toml
[dependencies]
revm-trace = { version = "3.0.0", default-features = false, features = ["rustls-tls"] }
```

To run examples with rustls-tls:
```bash
cargo run --example test_rustls --no-default-features --features rustls-tls
```

This is particularly useful for:
- Cross-compilation scenarios
- Environments where OpenSSL is not available
- Alpine Linux-based Docker containers
- WASM targets

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
revm-trace = "3.0.0"
```

## Quick Start

REVM-Trace v3.0 provides two distinct EVM modes to match your specific use case:

### 🚀 Mode 1: Standard EVM (High Performance)

Use `create_evm()` when you need **maximum speed** and only require execution results:

```rust
use revm_trace::{
    create_evm, 
    types::{SimulationTx, SimulationBatch},
};
use alloy::primitives::{address, U256, TxKind};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create high-performance EVM (no tracing overhead)
    let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;

    // Create simulation transaction
    let tx = SimulationTx {
        caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
        transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
        value: U256::from(120000000000000000u64), // 0.12 ETH
        data: vec![].into(),
    };

    let batch = SimulationBatch {
        block_env: None,
        transactions: vec![tx],
        is_stateful: false,
    };

    // ⚡ Ultra-fast execution - perfect for high-throughput scenarios
    let results = evm.execute_batch(batch);
    
    for result in results {
        match result {
            Ok(execution_result) => {
                println!("✅ Transaction succeeded!");
                println!("Gas used: {}", execution_result.gas_used());
            }
            Err(e) => println!("❌ Transaction failed: {}", e),
        }
    }

    Ok(())
}
```

### 🔍 Mode 2: Tracing EVM (Full Analysis)

Use `create_evm_with_tracer()` when you need **comprehensive analysis** with detailed trace data:

```rust
use revm_trace::{
    create_evm_with_tracer,
    TxInspector,
    types::{SimulationTx, SimulationBatch},
    traits::TransactionTrace,
};
use alloy::primitives::{address, U256, TxKind};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create tracing EVM with comprehensive analysis
    let tracer = TxInspector::new();
    let mut evm = create_evm_with_tracer(
        "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
        tracer
    ).await?;

    let tx = SimulationTx {
        caller: address!("C255fC198eEdAC7AF8aF0f6e0ca781794B094A61"),
        transact_to: TxKind::Call(address!("d878229c9c3575F224784DE610911B5607a3ad15")),
        value: U256::from(120000000000000000u64), // 0.12 ETH
        data: vec![].into(),
    };

    let batch = SimulationBatch {
        block_env: None,
        transactions: vec![tx],
        is_stateful: false,
    };

    // 🔍 Full tracing with detailed analysis
    let results = evm.trace_transactions(batch);
    
    for result in results {
        match result {
        match result {
            Ok((execution_result, trace_output)) => {
                println!("✅ Transaction succeeded with full trace!");
                println!("Gas used: {}", execution_result.gas_used());
                
                // 📊 Rich trace data analysis
                for transfer in trace_output.asset_transfers {
                    println!(
                        "💰 Transfer: {} from {} to {:?}",
                        transfer.value, transfer.from, transfer.to
                    );
                }
                
                // 📝 Complete call trace information
                println!("📊 Call depth: {}", trace_output.call_traces.len());
            }
            Err(e) => println!("❌ Transaction failed: {}", e),
        }
    }

    Ok(())
}
```

### 🌐 WebSocket Support

Both EVM modes support WebSocket connections for real-time blockchain data:

```rust
use revm_trace::{create_evm_ws, create_evm_ws_with_tracer, TxInspector};

// High-performance EVM with WebSocket
let evm = create_evm_ws("wss://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;

// Full tracing EVM with WebSocket  
let tracer = TxInspector::new();
let evm = create_evm_ws_with_tracer("wss://eth-mainnet.g.alchemy.com/v2/your-api-key", tracer).await?;
```
## 🔧 Batch Contract Calls with Multicall

The library includes universal Multicall support that works on any EVM-compatible chain:

```rust
use revm_trace::{
    create_evm,  // Use high-performance mode for batch calls
    utils::multicall_utils::{MulticallManager, MulticallCall},
};
use alloy::primitives::{address, Bytes};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create standard EVM (perfect for batch operations)
    let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;
    
    // Create multicall manager
    let multicall = MulticallManager::new();
    
    // Define batch calls
    let calls = vec![
        MulticallCall {
            target: address!("A0b86a33E6417c6d87c632B8de2C6D1Ce31A67Ba"), // USDC
            callData: Bytes::from(/* balanceOf call data */),
        },
        MulticallCall {
            target: address!("dAC17F958D2ee523a2206206994597C13D831ec7"), // USDT  
            callData: Bytes::from(/* balanceOf call data */),
        },
    ];
    
    // Execute batch calls with automatic deployment
    let results = multicall.deploy_and_batch_call(
        &mut evm,
        calls,
        false, // Allow individual call failures
        None,  // Use current block
    )?;
    
    // Process results
    for (i, result) in results.iter().enumerate() {
        if result.success {
            println!("Call {}: Success - {:?}", i, result.returnData);
        } else {
            println!("Call {}: Failed", i);
        }
    }

    Ok(())
}
```

## 🚀 Multi-Threading & Concurrent Processing

REVM-Trace v3.0 is designed from the ground up for **high-performance concurrent processing**:

```rust
use revm_trace::{create_evm, types::SimulationBatch};
use std::sync::Arc;
use tokio::task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 🔥 Each task gets its own high-performance EVM instance
    let mut handles = vec![];
    
    for i in 0..10 {
        let handle = task::spawn(async move {
            // Create dedicated EVM instance per thread (recommended pattern)
            let mut evm = create_evm("https://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;
            let batch = create_simulation_batch(i); // Your batch creation logic
            evm.execute_batch(batch)
        });
        handles.push(handle);
    }
    
    // Collect all results concurrently
    for handle in handles {
        let results = handle.await??;
        // Process results...
    }
    
    Ok(())
}
```

### 🎯 Performance Tips

| Scenario | Recommended Mode | Why |
|----------|------------------|-----|
| High-frequency trading simulations | `create_evm()` | Maximum speed, minimal overhead |
| DeFi protocol analysis | `create_evm_with_tracer()` | Rich trace data for comprehensive analysis |  
| Batch processing | `create_evm()` + concurrent tasks | Optimal throughput |
| Transaction debugging | `create_evm_with_tracer()` | Detailed error traces and call stacks |
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

## 🛡️ Thread Safety in v3.0

### ✅ What's New: Built-in Multi-Threading Support

REVM-Trace v3.0 introduces **native multi-threading capabilities** with optimized concurrent processing patterns:

```rust
use revm_trace::{create_evm, create_evm_with_tracer, TxInspector};
use tokio::task;

// ✅ Recommended: Each task creates its own EVM instance
async fn concurrent_processing() -> anyhow::Result<()> {
    let handles: Vec<_> = (0..10)
        .map(|i| {
            task::spawn(async move {
                // Each thread gets optimized EVM instance
                let mut evm = create_evm("https://rpc-url").await?;
                // Process transactions...
                Ok(())
            })
        })
        .collect();

    // Await all concurrent tasks
    for handle in handles {
        handle.await??;
    }
    
    Ok(())
}
```

### 🚀 Performance Patterns

| Pattern | v3.0 Recommendation | Performance |
|---------|-------------------|-------------|
| **Single-threaded** | `create_evm()` or `create_evm_with_tracer()` | ⭐⭐⭐ Good |
| **Multi-threaded** | One EVM per thread | ⭐⭐⭐⭐⭐ Excellent |
| **High-throughput** | `create_evm()` + concurrent tasks | ⭐⭐⭐⭐⭐ Maximum |

### ⚠️ Migration from v2.x

In v2.x, EVM instances required careful handling for concurrency. **v3.0 eliminates these concerns**:

```rust
// ❌ v2.x: Complex shared state management
// let evm = Arc::new(Mutex::new(create_evm().await?));

// ✅ v3.0: Simple per-thread instances
let mut evm = create_evm("https://rpc-url").await?;
```
## 🛡️ Safe Simulation Environment

All simulations run in an isolated environment:
- ✅ No actual blockchain state is modified
- ✅ No real transactions are submitted  
- ✅ No gas fees are spent
- ✅ Perfect for testing and validation
- ✅ Full rollback support for complex scenarios

### 📈 Performance Considerations

- **RPC Optimization**: Each EVM instance maintains optimized RPC connections
- **Memory Efficiency**: Smart caching reduces memory footprint
- **Concurrent Processing**: Built-in support for high-throughput scenarios
- **Resource Management**: Automatic cleanup and connection pooling

**Recommended Patterns**:
- Small batches: Use single EVM instance with `execute_batch()`
- Large batches: Use multiple EVM instances across threads  
- Real-time processing: Use WebSocket connections with `create_evm_ws()`
  
    

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

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

Built with ❤️ using:
- [REVM](https://github.com/bluealloy/revm) - The Rust Ethereum Virtual Machine
- [Alloy](https://github.com/alloy-rs/alloy) - High-performance Ethereum library

---

**REVM-Trace v3.0** - *Multi-threaded EVM simulation with comprehensive analysis*

