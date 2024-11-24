# REVM Transaction Simulator and Asset Tracer

A Rust library for simulating EVM-compatible blockchain transactions and tracking asset transfers (native tokens and ERC20 tokens) using REVM. This library provides a safe and efficient way to simulate transactions and analyze their effects without actually submitting them to the blockchain.

## Features

- **Transaction Simulation**: Simulate transactions on any EVM-compatible chain (Ethereum, BSC, Polygon, etc.)
- **Asset Transfer Tracking**: Track both native tokens (ETH, BNB, MATIC, etc.) and ERC20 token transfers
- **Token Information**: Automatically collect token symbols and decimals
- **Proxy Support**: Handle proxy contracts with implementation resolution
- **Safe Execution**: Simulate transactions without affecting the blockchain
- **Flexible Inspector**: Customizable asset transfer tracking

## Installation

Add this to your `Cargo.toml`:
```toml
revm-trace = "0.1.0"
```

## Quick Start

```rust
use revm_trace::{
  trace_tx_assets,
  TransactionTracer,
  create_evm_instance_with_inspector,
};
async fn example() -> anyhow::Result<()> {
  // Create EVM instance with transaction tracer
  let inspector = TransactionTracer::default();
  
  let mut evm = create_evm_instance_with_inspector(
    "https://rpc.ankr.com/eth", // Can be any EVM-compatible chain RPC
    inspector,
    None  // Use latest block
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
```

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
