
# REVM Transaction Simulator and Analyzer v4.1

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
- Security auditing and attack analysis: Track every storage slot read/write, reconstruct the full mutation history of any transaction, and analyze complex exploits or Safe wallet operations with unprecedented detail.

---


## 🚀 What's New in v4.1.0

- **Comprehensive Storage Slot Access Tracking**: All transaction traces now include a unified, global record of every storage slot read and write (SlotAccess), with full details (slot address, old/new value, access type, call context, etc.).
- **Recursive Call Trace Slot Access**: Each call trace (`CallTrace`) now recursively collects all slot accesses (read/write/all), supporting deep contract call trees and complex exploit analysis.
- **Type Filtering API**: Easily filter slot accesses by type (read/write/all) for both global and per-call-trace analysis, enabling precise state mutation auditing.
- **Security & Audit Ready**: Instantly reconstruct the full mutation history of any transaction, analyze Safe wallet operations, privilege escalations, and hacker attacks with unprecedented granularity.
- **Backward Compatible**: Existing simulation, asset transfer, and event tracing APIs remain unchanged—slot access tracking is fully additive.

---

### v4.0 Highlights (for reference)

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
  // This will also create a brand new EVM environment and clears all internal cache.
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
- **Comprehensive Storage Slot Tracking**: Every transaction now returns a global record of all storage slot changes (SlotAccess), and each call trace records every slot read/write (with type filtering and unified structure). This enables full reconstruction of storage mutation history for any transaction.
- **Security & Audit Ready**: Instantly see which slots were read or written, in what order, and by which contract call—ideal for analyzing hacker attacks, privilege escalations, Safe wallet operations, and more.
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
- **Full Storage Slot Access History**: For every transaction and every call trace, records all storage slot reads and writes (SlotAccess), including slot address, old/new value, and access type (read/write). This enables:
    - Step-by-step reconstruction of how a hacker or contract modifies storage
    - Auditing Safe wallet transactions for unexpected or suspicious state changes
    - Deep debugging of DeFi protocols and complex contract logic
- **Error Investigation Tools**: Locates exact failure points in complex call chains, decodes revert reasons, and provides contract-specific error context.
- **Performance**: Optimized for both single transaction and batch processing scenarios.

---

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
revm-trace = "4.1.0"
```

### TLS Backend Selection

**Important**: The TLS backend features are mutually exclusive. Choose only one:

```toml
# Option 1: Default - uses native-tls (OpenSSL) for maximum compatibility
revm-trace = "4.1.0"

# Option 2: Pure Rust TLS with rustls for system-dependency-free builds
revm-trace = { version = "4.1.0", default-features = false, features = ["rustls-tls"] }
```

---


## Quick Example

Simulate an ERC20 transfer and print slot changes in just a few lines:

```rust
use revm_trace::{create_evm_with_tracer, types::SlotAccessType, SimulationTx, SimulationBatch, TxInspector};
use alloy::primitives::{address, U256, TxKind};

#[tokio::main]
async fn main() {
    let inspector = TxInspector::new();
    let mut evm = create_evm_with_tracer("https://eth.llamarpc.com", inspector).await.unwrap();
    let tx = SimulationTx {
        caller: address!("28C6c06298d514Db089934071355E5743bf21d60"),
        transact_to: TxKind::Call(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48")),
        value: U256::ZERO,
        data: hex::decode("a9059cbb00000000000000000000000034e5dacdc16ff5bcdbdfa66c21a20f46347d86cf00000000000000000000000000000000000000000000000000000000000f4240").unwrap().into(),
    };
    let result = &evm.trace_transactions(SimulationBatch {
        is_stateful: false,
        transactions: vec![tx],
    }).into_iter().map(|v| v.unwrap()).collect::<Vec<_>>()[0];
    // Print all slot writes in the call trace
    if let Some(call_trace) = result.2.call_trace.as_ref() {
        for change in call_trace.all_slot_accesses(SlotAccessType::Write) {
            println!("Slot Write: {:?}", change);
        }
    }
}
```

See [examples/](./examples/) for advanced usage: multi-threaded simulation, custom inspectors, batch processing, DeFi/security analysis, and more.

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

**REVM-Trace v4.1.0** - *Multi-threaded EVM simulation with comprehensive analysis*

