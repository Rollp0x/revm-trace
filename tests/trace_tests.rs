//! Integration tests for transaction tracing and asset tracking
//!
//! This test module verifies the transaction simulation and asset tracing functionality
//! across different scenarios:
//!
//! # Test Coverage
//! - Historical state access at different block heights
//! - Different caller types (EOA vs Contract addresses)
//! - Different inspector configurations
//! - UniswapV2 swap transaction simulation
//!
//! # Test Infrastructure
//! - Uses Ankr's public RPC endpoint
//! - Requires multi-threaded tokio runtime
//! - Tests both successful and failure cases
//!
//! # Note on Historical State Access
//! The tests include scenarios for accessing historical state, but success depends
//! on the RPC node's capabilities:
//! - Recent blocks (WithinRange): May succeed on regular nodes
//! - Old blocks (OutOfRange): Requires archive node access
//!
//! Run tests with:
//! ```bash
//! cargo test --test trace_tests -- --nocapture
//! ```

use revm_trace::{
    create_evm_instance, create_evm_instance_with_tracer, trace_tx_assets, BlockEnvConfig,
};

use alloy::{
    primitives::{address, Address, U256},
    providers::{Provider, ProviderBuilder},
    sol,
    sol_types::SolCall,
};

sol! {
    contract UniswapV2Router {
        function swapExactETHForTokens(
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external payable returns (uint256[] memory amounts);
    }
}

/// Block selection mode for testing different historical state access scenarios
#[derive(Debug)]
enum BlockMode {
    /// Use the latest block
    Latest,
    /// Use a recent block (within node's historical state range)
    /// Note: The actual accessible range depends on the node configuration:
    /// - Regular nodes: typically ~128 blocks
    /// - Archive nodes: unlimited historical access
    WithinRange,
    /// Use a block far in the past (beyond typical node's historical state range)
    /// Expects to fail on regular nodes, may succeed on archive nodes
    OutOfRange,
}



async fn test_uniswap_swap_trace(
    caller: Address,
    description: &str,
    with_trace:bool,
    block_mode: BlockMode,
) 
{

    // Create provider and get current block
    let provider = ProviderBuilder::new()
        .on_http("https://rpc.ankr.com/eth".parse().unwrap());
    let latest_block = provider.get_block_number().await.unwrap();
    println!("Current block number: {}", latest_block);
    let block_number = match block_mode {
        BlockMode::Latest => latest_block,
        BlockMode::WithinRange => latest_block - 120,
        BlockMode::OutOfRange => latest_block - 10000,
    };

    let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"); // Uniswap V2 Router
    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let usdc = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");

    // Construct swapExactETHForTokens call
    let path = vec![weth, usdc];
    let deadline = U256::from(u64::MAX);
    let data = UniswapV2Router::swapExactETHForTokensCall {
        amountOutMin: U256::ZERO,
        path,
        to: caller,
        deadline,
    }
    .abi_encode();

    let result =if with_trace {
        let mut evm = create_evm_instance_with_tracer(
        "https://rpc.ankr.com/eth",
        Some(1)
        ).unwrap();
        evm.set_block_number(block_number);
        trace_tx_assets(
            &mut evm,
            caller,
            router,
            U256::from(100000000000000000u128), // 0.1 ETH
            data.into(),
            "ETH",
        )
        .await
    } else {
        let mut evm = create_evm_instance(
            "https://rpc.ankr.com/eth",
            Some(1)
        ).unwrap();
        evm.set_block_number(block_number);
        trace_tx_assets(
            &mut evm,
            caller,
            router,
            U256::from(100000000000000000u128), // 0.1 ETH
            data.into(),
            "ETH",
        )
        .await
    };
    
    // Adjust validation logic based on inspector type
    if !with_trace {
        assert!(
            !result.asset_transfers().is_empty(),
            "No native token transfers found for {}",
            description
        );
        let eth_transfers = result.asset_transfers();
        assert_eq!(
            eth_transfers.len(),
            1,
            "Should have exactly one ETH transfer"
        );
        assert_eq!(eth_transfers[0].from, caller);
        assert_eq!(eth_transfers[0].to, router);
        assert_eq!(eth_transfers[0].value, U256::from(100000000000000000u128));
    } else {
        assert!(
            result.asset_transfers().len() > 2,
            "No native token transfers found for {}",
            description
        );
        let usdc_info = result.token_info.get(&usdc).expect("Should have USDC info");
        assert_eq!(usdc_info.symbol, "USDC");
        assert_eq!(usdc_info.decimals, 6);
    }
    println!(
        "Token transfers for {}: {:#?}",
        description,
        result.asset_transfers()
    );
}

/// Test asset tracing with EOA caller using the latest block
/// Uses TransactionTracer to track ETH -> USDC swap on Uniswap V2
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_eoa() {
    let from = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    test_uniswap_swap_trace(from, "EOA", true, BlockMode::Latest).await;
}

/// Test asset tracing with EOA caller using a recent historical block
/// Verifies tracing functionality works with blocks within node's state range
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_eoa_within_range() {
    let from = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    test_uniswap_swap_trace(
        from,
        "EOA",
        true,
        BlockMode::WithinRange,
    )
    .await;
}

/// Test asset tracing with EOA caller using an old block
/// Expected to fail on regular nodes, may succeed on archive nodes
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_eoa_out_of_range() {
    let from = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    test_uniswap_swap_trace(
        from,
        "EOA",
        true,
        BlockMode::OutOfRange,
    )
    .await;
}

/// Test asset tracing with contract caller using the latest block
/// Simulates a contract-initiated swap on Uniswap V2
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_contract() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "Contract",
        true,
        BlockMode::Latest,
    )
    .await;
}

/// Test asset tracing with contract caller using an old block
/// Expected to fail on regular nodes, may succeed on archive nodes
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_contract_out_of_range() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "Contract",
        true,
        BlockMode::OutOfRange,
    )
    .await;
}

/// Test asset tracing with contract caller using a recent historical block
/// Verifies tracing functionality works with blocks within node's state range
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_with_contract_within_range() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "Contract",
        true,
        BlockMode::WithinRange,
    )
    .await;
}

/// Test basic transaction simulation without asset tracing
/// Uses NoOpInspector with latest block
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_without_inspector() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "No Inspector",
        false,
        BlockMode::Latest,
    )
    .await;
}

/// Test basic transaction simulation without asset tracing
/// Uses NoOpInspector with latest block
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_without_inspector_with_eoa() {
    let from = address!("57757E3D981446D585Af0D9Ae4d7DF6D64647806");
    test_uniswap_swap_trace(
        from,
        "No Inspector",
        false,
        BlockMode::Latest,
    )
    .await;
}

/// Test basic transaction simulation without asset tracing using a recent block
/// Verifies basic EVM simulation works with blocks within node's state range
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_without_inspector_within_range() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "No Inspector",
        false,
        BlockMode::WithinRange,
    )
    .await;
}

/// Test basic transaction simulation without asset tracing using an old block
/// Expected to fail on regular nodes, may succeed on archive nodes
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_tx_assets_complex_without_inspector_out_of_range() {
    let from = address!("3ee18B2214AFF97000D974cf647E7C347E8fa585");
    test_uniswap_swap_trace(
        from,
        "No Inspector",
        false,
        BlockMode::OutOfRange,
    )
    .await;
}
