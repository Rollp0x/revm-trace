# REVM Transaction Simulator and Analyzer v3.1.1

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

## 🚀 What's New in v3.1.1

- **🎯 Enhanced TxInspector Support**: Specialized `get_inspector()` method for direct access to all TxInspector functionality
- **📚 Comprehensive Documentation**: Clear explanation of three usage modes and modern REVM API requirements
- **🔧 Advanced Error Tracing**: Full access to `get_error_trace_address()`, `find_error_trace()`, and all inspector methods
- **⚡ Optimized API Design**: Perfect balance between generic flexibility and TxInspector-specific convenience

## 🎯 Core Value: TxInspector

**TxInspector** is our flagship feature - a comprehensive transaction analyzer that provides:

- **Complete Asset Transfer Tracking**: ETH and ERC20 transfers with full context
- **Advanced Call Tree Analysis**: Hierarchical call structure with precise error location
- **Event Log Collection**: All emitted events with automatic parsing
- **Error Investigation Tools**: Pinpoint exact failure locations in complex call chains
- **High-Performance Design**: Optimized for both single transactions and batch processing

## Key Features

- **Three Usage Modes for Every Scenario**
  - **Simple Execution**: Ultra-fast gas estimation (`create_evm()`)
  - **Manual Control**: Advanced tracing with fine-grained control (`create_evm_with_tracer()` + manual)
  - **Automatic Processing**: Convenience API with automatic state management (`trace_transactions()`)

- **TxInspector Specialized Support**
  - Direct access to all inspector methods without type erasure
  - Advanced error tracing and call analysis
  - Seamless integration with modern REVM architecture

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

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
revm-trace = "3.1.1"
```

### TLS Backend Selection

**Important**: The TLS backend features are mutually exclusive. Choose only one:

```toml
[dependencies]
# Option 1: Default - uses native-tls (OpenSSL) for maximum compatibility
revm-trace = "3.1.1"

# Option 2: Pure Rust TLS with rustls for system-dependency-free builds
revm-trace = { version = "3.1.1", default-features = false, features = ["rustls-tls"] }
```

Do not specify both features simultaneously, as this will include both TLS implementations and increase binary size unnecessarily.

## 🎯 Three Usage Modes

REVM-Trace provides three distinct usage patterns to suit different scenarios:

### 1. 🚀 Simple Execution Mode (No Tracing)
- **Use case**: Gas estimation, basic simulation, high-throughput scenarios
- **Inspector**: `NoOpInspector` (output: `()`)
- **Performance**: Fastest - zero tracing overhead  
- **API**: `create_evm()` + `execute_batch()`

### 2. 🔧 Manual Inspector Control (Advanced)
- **Use case**: Custom tracing logic, debugging, research, fine-grained control
- **Inspector**: Any custom inspector (e.g., `TxInspector`)
- **Performance**: Full control over data collection and state management
- **API**: `create_evm_with_tracer()` + manual `inspect_replay_commit()`

### 3. 🎯 Automatic Batch Processing (Convenience)
- **Use case**: Standard trace analysis, automated workflows
- **Inspector**: Must implement `TraceOutput` trait (e.g., `TxInspector`)
- **Performance**: Automatic state management with predictable overhead
- **API**: `create_evm_with_tracer()` + `trace_transactions()`

## ⚠️ Critical REVM API Changes

**Modern REVM Requirement**: Inspector execution must be explicitly activated!

- ❌ **Old REVM**: `evm.transact(tx)` would automatically trigger Inspector hooks
- ✅ **New REVM**: `evm.transact(tx)` does **NOT** execute Inspector - you get raw execution only
- ✅ **New REVM**: Must call `evm.inspect_replay_commit()` to activate Inspector

This change enables:
- Better performance control (skip tracing when not needed)
- Explicit separation between execution and analysis  
- Flexible inspector activation patterns

**Key Point**: The convenience functions like `trace_transactions()` automatically handle `inspect_replay_commit()` for you, but manual mode requires explicit activation.

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

### 🔧 Mode 3: Manual Inspector Control (Advanced)

For users who need fine-grained control over the tracing process:

```rust
use revm_trace::{create_evm_with_tracer, TxInspector};
use alloy::primitives::{address, U256, TxKind};
use revm::context::TxEnv;
use revm::{ExecuteEvm, InspectCommitEvm};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", inspector).await?;

    // Manual workflow: Full control over inspector
    let tx = TxEnv::builder()
        .caller(address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"))
        .kind(TxKind::Call(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")))
        .value(U256::ZERO)
        .build_fill();

    // Step 1: Set transaction
    evm.set_tx(tx);

    // Step 2: CRITICAL - Explicit Inspector activation (modern REVM requirement!)
    // Note: evm.transact() would NOT execute the inspector
    let result = evm.inspect_replay_commit()?;

    // Step 3: Access TxInspector-specific methods anytime
    let inspector = evm.get_inspector();
    let transfers = inspector.get_transfers();
    let traces = inspector.get_traces();
    let logs = inspector.get_logs();
    let error_location = inspector.get_error_trace_address();

    println!("🔍 Transfers: {}, Traces: {}, Logs: {}", 
             transfers.len(), traces.len(), logs.len());

    if let Some(error_addr) = error_location {
        println!("❌ Error at: {:?}", error_addr);
        if let Some(error_trace) = inspector.find_error_trace() {
            println!("Failed call: {:?} -> {:?}", error_trace.call_scheme, error_trace.status);
        }
    } else {
        println!("✅ All calls executed successfully");
    }

    // Step 4: Manual state management (optional)
    evm.reset_inspector();  // Clear for next transaction

    Ok(())
}
```

### 🌐 WebSocket Support

Both EVM modes support WebSocket connections for real-time blockchain data:

```rust
use revm_trace::{create_evm, create_evm_with_tracer, TxInspector};

// High-performance EVM with WebSocket (auto-detected from URL)
let evm = create_evm("wss://eth-mainnet.g.alchemy.com/v2/your-api-key").await?;

// Full tracing EVM with WebSocket (auto-detected from URL)
let tracer = TxInspector::new();
let evm = create_evm_with_tracer("wss://eth-mainnet.g.alchemy.com/v2/your-api-key", tracer).await?;
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

## 🤔 Which Mode Should I Use?

### Mode Selection Guide

| Scenario | Recommended Mode | API | Reason |
|----------|------------------|-----|--------|
| 🚀 Gas estimation, basic simulation | **Mode 1** | `create_evm()` + `execute_batch()` | Fastest, zero overhead |
| 🔧 Custom tracing, debugging, research | **Mode 2** | `create_evm_with_tracer()` + manual control | Full control, all inspector methods |
| 🎯 Standard trace analysis, automation | **Mode 3** | `create_evm_with_tracer()` + `trace_transactions()` | Clean API, automatic management |
| 🔄 High-throughput processing | **Mode 1** or **Mode 2** | Concurrent instances | Avoid TraceOutput overhead |
| 🧪 Inspector development | **Mode 2** | Manual `inspect_replay_commit()` | Direct access to internals |
| 📊 DeFi analysis & reporting | **Mode 3** | `trace_transactions()` | Rich data with automatic cleanup |

### Key Decision Factors

- **Need tracing data?** → Use Mode 2 or 3
- **Want automatic management?** → Use Mode 3  
- **Need maximum performance?** → Use Mode 1
- **Building custom inspector?** → Use Mode 2
- **Processing thousands of transactions?** → Use Mode 1 + concurrency

### 🎯 Performance Tips

| Scenario | Recommended Mode | Why |
|----------|------------------|-----|
| High-frequency trading simulations | `create_evm()` | Maximum speed, minimal overhead |
| DeFi protocol analysis | `create_evm_with_tracer()` | Rich trace data for comprehensive analysis |  
| Batch processing | `create_evm()` + concurrent tasks | Optimal throughput |
| Transaction debugging | `create_evm_with_tracer()` | Detailed error traces and call stacks |


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

In v2.x, EVM instances were **NOT thread-safe and could NOT be used concurrently**, even with `Arc<Mutex<>>` wrapping due to underlying database connection limitations. **v3.0 introduces true multi-threading support**:

```rust
// ❌ v2.x: NO concurrent support - this would fail!
// let evm = Arc::new(Mutex::new(create_evm().await?)); // ← This doesn't work!
// Multiple threads would cause database connection conflicts

// ✅ v3.0: True multi-threading - each thread gets its own optimized EVM
let mut evm = create_evm("https://rpc-url").await?;  // Thread-safe from ground up
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
- Real-time processing: Use WebSocket connections with `create_evm("wss://...")"`
  
    

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

## 🌐 Web API Integration

### Actix-Web Integration

REVM-Trace v3.0 provides seamless integration with web frameworks. Here's a complete example using [Actix-Web](https://actix.rs/):

```rust
// examples/actix_web_integration.rs
use actix_web::{web, App, HttpServer, HttpResponse, Result};
use revm_trace::{create_evm, create_evm_with_tracer, TxInspector};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/simulate", web::post().to(simulate_transaction))
            .route("/health", web::get().to(health_check))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

#### Key Features:
- **🚀 Two Implementation Approaches**: Choose between `tokio::task::spawn_blocking` or `web::block`
- **⚡ High Performance**: Each request creates optimized EVM instances
- **🛡️ Thread Safety**: Built-in multi-threading support for concurrent requests
- **📊 Flexible Responses**: Optional tracing with detailed call traces and asset transfers
- **🔧 Easy Integration**: Drop-in solution for existing Actix-Web applications

#### API Example:

**Request:**
```json
{
    "rpc_url": "https://eth.llamarpc.com",
    "from": "0xC255fC198eEdAC7AF8aF0f6e0ca781794B094A61",
    "to": "0xd878229c9c3575F224784DE610911B5607a3ad15",
    "value": "120000000000000000",
    "data": "0x",
    "with_trace": true
}
```

**Response:**
```json
{
    "success": true,
    "gas_used": 21000,
    "error": null,
    "traces": {
        "asset_transfers": 1,
        "call_traces": {
            "from": "0xc255fc198eedac7af8af0f6e0ca781794b094a61",
            "to": "0xd878229c9c3575f224784de610911b5607a3ad15",
            "value": "0x1aa535d3d0c0000",
            "call_scheme": "Call",
            "gas_used": "0x0",
            "status": "Success"
        }
    }
}
```

#### Multi-Threading Approaches:

**Approach 1: `web::block` (Recommended)**
```rust
async fn simulate_transaction(req: web::Json<SimulateRequest>) -> Result<HttpResponse> {
    let request = req.into_inner();
    
    let result = web::block(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async { simulate_tx_internal(request).await })
    }).await;
    
    // Handle result...
}
```

**Approach 2: `tokio::task::spawn_blocking`**
```rust
async fn simulate_transaction(req: web::Json<SimulateRequest>) -> Result<HttpResponse> {
    let request = req.into_inner();
    
    let result = tokio::task::spawn_blocking(move || {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            simulate_tx_internal(request).await
        })
    }).await;
    
    // Handle result...
}
```

**Run the Example:**
```bash
cargo run --example actix_web_integration

# Test with curl:
curl -X POST http://127.0.0.1:8080/simulate \
  -H "Content-Type: application/json" \
  -d '{"rpc_url":"https://eth.llamarpc.com","from":"0xC255fC198eEdAC7AF8aF0f6e0ca781794B094A61","to":"0xd878229c9c3575F224784DE610911B5607a3ad15","value":"120000000000000000","with_trace":true}'
```

> **💡 Production Tip**: Consider using Nginx rate limiting for production deployments to manage request frequency and prevent resource exhaustion.

---

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
- [Foundry Fork DB](https://github.com/foundry-rs/foundry-fork-db) - Efficient blockchain state forking and caching

---

**REVM-Trace v3.0** - *Multi-threaded EVM simulation with comprehensive analysis*

