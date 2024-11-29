# REVM Transaction Simulator and Asset Tracer

A Rust library for simulating EVM-compatible blockchain transactions and tracking asset transfers, logs, and events using REVM. This library provides a safe and efficient way to simulate transactions and analyze their effects without actually submitting them to the blockchain.

## Features

- **Transaction Simulation**: Simulate transactions on any EVM-compatible chain (Ethereum, BSC, Polygon, etc.)
- **Asset Transfer Tracking**: Track both native tokens (ETH, BNB, MATIC, etc.) and ERC20 token transfers
- **Event Collection**: Capture and analyze all transaction logs and events
- **Token Information**: Automatically collect token symbols and decimals
- **Proxy Support**: Handle proxy contracts with implementation resolution
- **Safe Execution**: Simulate transactions without affecting the blockchain
- **Flexible Inspector**: Customizable transaction tracing

## Installation

Add this to your `Cargo.toml`:
```toml
revm-trace = "1.0.0"
```

## Quick Start

```rust
use revm_trace::{
  trace_tx_assets,
  create_evm_instance_with_tracer,
};
async fn example() -> anyhow::Result<()> {
  // Create EVM instance with transaction tracer
  let inspector = TransactionTracer::default();
  
  let mut evm = create_evm_instance_with_tracer(
    "https://rpc.ankr.com/eth", // Can be any EVM-compatible chain RPC
    Some(1) // Chain ID: 1 for Ethereum mainnet
  )?;
  
  // Simulate transaction and track transfers
  let result = trace_tx_assets(
    &mut evm,
    from_address,
    to_address,
    value,
    call_data,
    "ETH"  // Native token symbol
  ).await;
  
  // Process results
  for transfer in result.asset_transfers() {
    if transfer.is_native_token() {
      println!("Native token transfer: {} -> {}: {}",
        transfer.from,
        transfer.to,
        transfer.value
      );
    } else {
      let token_info = result.token_info.get(&transfer.token)
        .expect("Token info should exist");
      println!("Token transfer: {} {} -> {}: {}",
        token_info.symbol,
        transfer.from,
        transfer.to,
        transfer.value
      );
    }
  }
  // Process logs
  for log in result.logs {
    println!("Log from {}: {:?}", log.address, log);
  }
  Ok(())
}
```

## Usage Examples

### Tracking DeFi Transactions

```rust
// Example of tracking a Uniswap-like DEX swap on any EVM chain
let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
let result = trace_tx_assets(
  &mut evm,
  user_address,
  router,
  native_token_amount,
  swap_data,
  "BNB"  // Use appropriate native token symbol
).await;
// Process transfers and logs
for transfer in result.asset_transfers() {
  println!("Transfer: {:?}", transfer);
}
for log in result.logs {
  if log.topics()[0] == SWAP_EVENT_SIGNATURE {
    println!("Swap event: {:?}", log);
  }
}
```



### Reusing EVM Instance

When simulating multiple transactions using the same EVM instance, you need to reset the inspector state between simulations:

```rust
use revm_trace::{
  trace_tx_assets,
  create_evm_instance_with_tracer,
  GetTransactionTracer,
  Reset,
};
async fn simulate_multiple_txs() -> anyhow::Result<()> {
  let mut evm = create_evm_instance_with_tracer(
    "https://rpc.ankr.com/eth",
    Some(1)
  )?;
  // First simulation
  let result1 = trace_tx_assets(&mut evm, from, to1, value1, data1, "ETH").await;
  // Reset inspector state before next simulation
  evm.reset_inspector();
  // Second simulation with clean state
  let result2 = trace_tx_assets(&mut evm, from, to2, value2, data2, "ETH").await;
  Ok(())
}

```

### Setting Block Environment

You can customize the block environment for simulation:

```rust
use revm_trace::{
  trace_tx_assets,
  create_evm_instance_with_tracer,
  BlockEnvConfig,
};
async fn simulate_at_specific_block() -> anyhow::Result<()> {
  let mut evm = create_evm_instance_with_tracer(
    "https://rpc.ankr.com/eth",
    Some(1)
  )?;
  // Set specific block number and timestamp
  evm.set_block_number(17_000_000)
  	.set_block_timestamp(1677777777);
  // Or set both at once
  evm.set_block_env(17_000_000, 1677777777);
  let result = trace_tx_assets(&mut evm, from, to, value, data, "ETH").await;
  Ok(())
}
```





### Thread Safety and Parallel Processing

#### Important Note on Thread Safety

The core `Evm` instance from REVM is not thread-safe, and consequently, our tracing functionality cannot be used across threads. This means you cannot share an `Evm` instance between threads or use it in parallel operations.

#### Recommended Usage Patterns

##### ❌ What to Avoid

```rust
// This will NOT work - do not share Evm across threads
let mut evm = create_evm_instance_with_tracer("https://rpc...", Some(1))?;
let handles: Vec<> = transactions
	.into_par_iter() // ❌ Parallel processing will fail
	.map(|tx| {
		trace_tx_assets(&mut evm, tx.from, tx.to, tx.value, tx.data, "ETH")
	})
	.collect();
```

##### ✅ Correct Usage

```rust
// Create separate EVM instances for each thread
let handles: Vec<> = transactions
  .into_iter() // ✅ Sequential processing
  .map(|tx| {
  	let mut evm = create_evm_instance_with_tracer("https://rpc...", Some(1))?;
  	trace_tx_assets(&mut evm, tx.from, tx.to, tx.value, tx.data, "ETH")
  })
  .collect();

// Alternative: If you need parallel processing
	use rayon::prelude::;
  let results: Vec<> = transactions
  .par_iter()
  .map(|tx| {
    let mut evm = create_evm_instance_with_tracer("https://rpc...", Some(1))?;
    trace_tx_assets(&mut evm, tx.from, tx.to, tx.value, tx.data, "ETH")
  })
  .collect();
```

#### Performance Considerations

- Create new EVM instances for parallel operations

- Consider connection pool limits of your RPC provider

- Balance between parallelism and RPC rate limits

  

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

When accessing historical blockchain state, capabilities depend on the node type:

- **Archive Nodes**: Can access any historical block state
- **Full Nodes**: Limited to recent blocks (typically ~128 blocks)

The actual accessible block range varies by provider and node configuration.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Built with [REVM](https://github.com/bluealloy/revm) and [Alloy](https://github.com/alloy-rs/alloy)

