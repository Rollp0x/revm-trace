# REVM Transaction Simulator and Analyzer v4.0

A high-performance, **multi-threaded** Rust library for EVM transaction simulation and analysis, built on [REVM](https://github.com/bluealloy/revm).

- **Simulate** complex transactions and their interactions before actual execution
- **Analyze** potential outcomes, asset transfers, and state changes
- **Detect** possible errors and their root causes
- **Preview** all transaction effects in a safe, isolated environment
- **Process** multiple transactions concurrently with built-in thread safety

Perfect for:
- DeFi developers testing complex interactions
- Safe wallet users validating Safe transaction safety
- Protocol teams analyzing contract behaviors
- Security researchers investigating transaction patterns
- High-throughput applications requiring concurrent transaction processing

---

## 🚀 What's New in v4.0

- **Unified EVM Construction with EvmBuilder**: Use `EvmBuilder` for full control (custom block height, inspector, etc.). For convenience, use `create_evm` and `create_evm_with_tracer` for quick EVM creation at the latest block.
- **Block Height Management**: Specify block height via builder pattern, or update after creation with `set_db_block` (which also resets the database cache to ensure state consistency).
- **Backend Selection**: Default backend is AlloyDB. Enable the `foundry-fork` feature for high-performance, thread-safe simulation with Foundry-fork-db (see `examples/concurrent_shared_backend.rs`).
- **Simplified API**: All utility functions no longer require a `block_env` parameter; block context is managed at EVM creation.
- **Breaking Changes**: EVM construction and block management APIs have changed. Please update your code to use the new builder pattern or context management methods.
- **NFT Transfer Support**: Unified parsing and tracing of ERC20, ERC721, and ERC1155 token transfers. The `TokenTransfer` struct now includes `token_type` and `id` fields to support NFTs.

---

## EVM Construction Patterns

- **Quick Start (Latest Block)**
  ```rust
  let mut evm = create_evm("https://eth.llamarpc.com").await?;
  // or with tracing:
  let tracer = TxInspector::new();
  let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", tracer).await?;
  ```

- **Custom Block Height (Recommended for Historical Simulation)**
  ```rust
  let mut evm = EvmBuilder::new_alloy("https://eth.llamarpc.com")
      .with_block_number(18_000_000)
      .with_tracer(TxInspector::new())
      .build()
      .await?;
  ```

- **Change Block Context After Creation**
  ```rust
  // After creating the EVM, you can update the block context:
  evm.set_db_block(block_env)?;
  ```

- **Multi-Threaded Simulation (Foundry-fork-db)**
  - Enable the `foundry-fork` feature in Cargo.toml.
  - See `examples/concurrent_shared_backend.rs` for a complete example.

---

### Usage Modes

| Mode | API | Inspector | Use Case | Performance |
|------|-----|-----------|----------|-------------|
| **1. Simple Execution** | `create_evm()` + `execute_batch()` | `NoOpInspector` | Gas estimation, fast simulation | Fastest, no tracing |
| **2. Manual Inspector** | `create_evm_with_tracer()` + manual `inspect_replay_commit()` | Custom (e.g. `TxInspector`) | Debugging, custom tracing, research | Full control |
| **3. Automatic Batch** | `create_evm_with_tracer()` + `trace_transactions()` | Must implement `TraceOutput` | Standard trace analysis, automation | Clean API, auto state mgmt |

- **Mode 1:** For high-throughput, no-tracing scenarios.
- **Mode 2:** For advanced users needing full inspector control.
- **Mode 3:** For most users needing standard tracing and batch processing.

---

### Key Features

- **Flexible EVM Construction**: Unified builder pattern for AlloyDB and Foundry-fork-db backends.
- **Customizable Inspector System**: Use built-in `TxInspector` or your own inspector for tracing and analysis.
- **Multi-Threaded & High-Performance**: Foundry-fork-db backend enables safe, concurrent simulation with shared cache.
- **Batch Processing & Asset Analysis**: Simulate and analyze multiple transactions, including asset transfers and call traces.
- **Safe Simulation**: All simulations are isolated—no real blockchain state is modified.
- **EVM-Compatible Chain Support**: Works with any EVM-compatible blockchain, not just Ethereum mainnet.
- **Rich Utility Functions**: Includes tools for batch querying token balances, simulating Multicall deployment and batch execution, and more.
- **Flexible Connection**: Supports both HTTP and WebSocket (ws/wss) endpoints for EVM construction.
- **NFT (ERC721 & ERC1155) Transfer Analysis**: Automatically detects and parses NFT transfers, including tokenId extraction and type distinction.

---

### Advanced Usage & Extensibility
- While `trace_transactions` provides a convenient batch simulation and tracing API for most use cases, advanced users can construct and control the EVM instance directly using REVM and this crate’s inspector system.
- The core value of this crate lies in the `TxInspector` and its output `TxTraceOutput`, which provide detailed, structured tracing of transaction execution, asset transfers, call trees, events, and errors.
- For custom analysis (e.g., storage slot changes, balance diffs, or other state introspection), users can run their own simulation loop, obtain `ResultAndState` from REVM, and combine it with `TxInspector` for maximum flexibility.

### TxInspector Highlights

- **Comprehensive Asset Transfer Tracking**: Automatically tracks ETH and ERC20 transfers with full context.
- **Advanced Call Tree Analysis**: Builds hierarchical call traces and pinpoints error locations.
- **Event Log Collection**: Captures and parses all emitted events during simulation.
- **Error Investigation Tools**: Locates exact failure points in complex call chains, decodes revert reasons, and provides contract-specific error context.
- **Performance**: Optimized for both single transaction and batch processing scenarios.

---

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
revm-trace = "4.0.2"
```

### TLS Backend Selection

**Important**: The TLS backend features are mutually exclusive. Choose only one:

```toml
# Option 1: Default - uses native-tls (OpenSSL) for maximum compatibility
revm-trace = "4.0.2"

# Option 2: Pure Rust TLS with rustls for system-dependency-free builds
revm-trace = { version = "4.0.2", default-features = false, features = ["rustls-tls"] }
```

---

## More Examples

See the [examples directory](./examples/) for:
- Multi-threaded simulation (`concurrent_shared_backend.rs`)
- Custom block height and inspector usage
- Batch processing and multicall
- DeFi, token, and proxy contract analysis

---

## Example: Simulate an ERC20 Token Transfer

Below is a complete example demonstrating how to simulate an ERC20 token transfer, track the result, and display transfer information with token details. This example works with both AlloyDB (default) and Foundry-fork-db (with the `foundry-fork` feature enabled):

```rust
use revm_trace::{
    TransactionTrace,
    utils::erc20_utils::get_token_infos,
    SimulationBatch, SimulationTx, TxInspector
};
use anyhow::Result;
use alloy::{
    primitives::{address, utils::format_units, Address, U256,TxKind}, 
    sol, sol_types::SolCall
};

#[cfg(not(feature = "foundry-fork"))]
use revm_trace::create_evm_with_tracer;

#[cfg(feature = "foundry-fork")]
use revm_trace::create_shared_evm_with_tracer;

// Define ERC20 interface for transfer function
sol!(
    contract ERC20 {
        function transfer(address to, uint256 amount) external returns (bool);
    }
);

fn encode_erc20_transfer(to: Address, amount: U256) -> Vec<u8> {
    ERC20::transferCall { to, amount }.abi_encode()
}

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(not(feature = "foundry-fork"))]
    println!("Using AlloyDB backend for EVM simulation");
    #[cfg(feature = "foundry-fork")]
    println!("Using Foundry fork backend for EVM simulation");
    let inspector = TxInspector::new();
    #[cfg(not(feature = "foundry-fork"))]
    let mut evm = create_evm_with_tracer(
        ETH_RPC_URL,
        inspector,
    ).await?;
    #[cfg(feature = "foundry-fork")]
    let mut evm = create_shared_evm_with_tracer(
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
        is_stateful: false,
        transactions: vec![tx],
    }).into_iter().map(|v| v.unwrap()).collect::<Vec<_>>()[0];
    let output = result.0.output().unwrap();
    assert!(output.len() == 32 && output[31] == 1,"❌ Expected transfer to succeed");
    // Print results
    for transfer in &result.1.asset_transfers {
        let token_info = &get_token_infos(&mut evm, &[transfer.token]).unwrap()[0];
        println!(
            "Transfer: {} {} -> {}: {}",
            token_info.symbol, transfer.from, transfer.to.unwrap(), format_units(transfer.value, token_info.decimals).unwrap()
        );
    }
    Ok(())
}
```

---

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

Built with ❤️ using:
- [REVM](https://github.com/bluealloy/revm) - The Rust Ethereum Virtual Machine
- [Alloy](https://github.com/alloy-rs/alloy) - High-performance Ethereum library
- [Foundry Fork DB](https://github.com/foundry-rs/foundry-fork-db) - Efficient blockchain state forking and caching

---

**REVM-Trace v4.0** - *Multi-threaded EVM simulation with comprehensive analysis*

