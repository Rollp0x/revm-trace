use alloy::primitives::{utils::format_units, Address};
use anyhow::Result;
use revm_trace::utils::erc20_utils::query_erc20_balance;

#[cfg(not(feature = "foundry-fork"))]
use revm_trace::create_evm;

#[cfg(feature = "foundry-fork")]
use revm_trace::create_shared_evm;

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

// USDC Address (Ethereum Mainnet)
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Concurrent ERC20 Balance Queries Example");
    println!("{}", "=".repeat(60));

    test_concurrent_erc20_queries().await?;

    println!("\n✅ Concurrent ERC20 query example completed successfully!");

    Ok(())
}

async fn test_concurrent_erc20_queries() -> Result<()> {
    println!("🔍 Testing Concurrent ERC20 balance queries");

    #[cfg(not(feature = "foundry-fork"))]
    println!("Using AlloyDB backend for EVM simulation");

    #[cfg(feature = "foundry-fork")]
    println!("Using Foundry fork backend for EVM simulation");

    // Binance addresses to query
    // These addresses are known to hold large amounts of USDC
    let addresses = vec![
        "0x28C6c06298d514Db089934071355E5743bf21d60", // Binance cold wallet
        "0x3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE", // Binance hot wallet
        "0xDFd5293D8e347dFe59E90eFd55b2956a1343963d", // Binance 8
        "0x56Eddb7aa87536c09CCc2793473599fD21A8b17F", // Binance 12
        "0x9696f59E4d72E237BE84fFD425DCaD154Bf96976", // Binance 13
    ];

    let token_address: Address = USDC_ADDRESS.parse()?;

    let mut handles = vec![];

    println!("🚀 Launching {} concurrent queries...", addresses.len());

    for (i, addr_str) in addresses.iter().enumerate() {
        let addr_str = addr_str.to_string();

        let handle = tokio::spawn(async move {
            #[cfg(not(feature = "foundry-fork"))]
            let mut evm = create_evm(ETH_RPC_URL).await?;

            #[cfg(feature = "foundry-fork")]
            let mut evm = create_shared_evm(ETH_RPC_URL).await?;

            let owner_address: Address = addr_str.parse()?;
            let balance = query_erc20_balance(&mut evm, token_address, owner_address)?;

            println!(
                "🏦 Thread {}: USDC Balance of {}: {}",
                i + 1,
                addr_str,
                balance
            );

            Ok::<_, anyhow::Error>((addr_str, balance))
        });

        handles.push(handle);
    }

    // wait for all tasks to complete
    let mut results = vec![];
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }

    // validate results
    if results.len() == addresses.len() {
        println!(
            "✅ All {} concurrent queries completed successfully!",
            results.len()
        );
    } else {
        println!(
            "⚠️  Warning: Only {}/{} queries completed",
            results.len(),
            addresses.len()
        );
    }

    // show summary
    println!("\n📊 Query Results Summary:");
    println!("{}", "-".repeat(80));
    for (addr, balance) in results {
        let formatted_balance = format_units(balance, 6).unwrap_or_else(|_| "Invalid".to_string());
        println!("📈 {} -> {} USDC", addr, formatted_balance);
    }

    Ok(())
}
