use anyhow::Result;
use alloy::primitives::{Address, U256,utils::format_units};
use revm_trace::{
    create_evm,
    utils::erc20_utils::query_erc20_balance,
};

const RPC_URL: &str = "https://eth.llamarpc.com";

// USDC 合约地址 (以太坊主网)
const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";  
// Binance 的地址 (已知持有大量代币)
const BINANCE_ADDRESS: &str = "0x28C6c06298d514Db089934071355E5743bf21d60";

const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Concurrent ERC20 Balance Queries Example");
    println!("{}", "=".repeat(60));
    
    // 运行多线程 ERC20 查询测试
    test_query_erc20_balance_multi_thread().await?;
    
    println!("\n{}", "=".repeat(60));
    
    // 运行并发查询测试
    test_concurrent_erc20_queries().await?;
    
    println!("\n✅ All concurrent ERC20 query examples completed successfully!");
    
    Ok(())
}

async fn test_query_erc20_balance_multi_thread() -> Result<()> {
    println!("🔍 Testing ERC20 balance query - Multi Thread Mode");
    let mut evm = create_evm(
        ETH_RPC_URL
    ).await?;

    let token_address: Address = USDC_ADDRESS.parse()?;
    let owner_address: Address = BINANCE_ADDRESS.parse()?;

    let balance = query_erc20_balance(&mut evm, token_address, owner_address, None)?;
    
    println!("✅ USDC Balance of {}: {}", BINANCE_ADDRESS, balance);
    println!("📊 Balance in human readable (6 decimals): {}", 
            format_units(balance, 6).unwrap_or_else(|_| "Invalid".to_string()));

    // 验证余额不为零
    if balance > U256::ZERO {
        println!("✅ Verified: Balance is non-zero as expected");
    } else {
        println!("⚠️  Warning: Balance is zero, which might be unexpected");
    }

    Ok(())
}

async fn test_concurrent_erc20_queries() -> Result<()> {
    println!("🔍 Testing Concurrent ERC20 balance queries");
    
    // 使用多个不同的地址进行并发查询
    let addresses = vec![
        BINANCE_ADDRESS,
        "0x3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE", // Binance 热钱包
        "0x28C6c06298d514Db089934071355E5743bf21d60", // Binance 冷钱包
        "0xDFd5293D8e347dFe59E90eFd55b2956a1343963d", // Binance 8
        "0x56Eddb7aa87536c09CCc2793473599fD21A8b17F", // Binance 12
    ];

    let token_address: Address = USDC_ADDRESS.parse()?;
    
    let mut handles = vec![];
    
    println!("🚀 Launching {} concurrent queries...", addresses.len());
    
    for (i, addr_str) in addresses.iter().enumerate() {
        let addr_str = addr_str.to_string();
        let rpc_url = RPC_URL.to_string();
        
        let handle = tokio::spawn(async move {
            let mut evm = create_evm(&rpc_url).await?;

            let owner_address: Address = addr_str.parse()?;
            let balance = query_erc20_balance(&mut evm, token_address, owner_address, None)?;
            
            println!("🏦 Thread {}: USDC Balance of {}: {}", i, addr_str, balance);
            
            Ok::<_, anyhow::Error>((addr_str, balance))
        });
        
        handles.push(handle);
    }

    // 等待所有查询完成
    let mut results = vec![];
    for handle in handles {
        let result = handle.await??;
        results.push(result);
    }

    // 验证所有查询都成功了
    if results.len() == addresses.len() {
        println!("✅ All {} concurrent queries completed successfully!", results.len());
    } else {
        println!("⚠️  Warning: Only {}/{} queries completed", results.len(), addresses.len());
    }

    // 显示结果摘要
    println!("\n📊 Query Results Summary:");
    println!("{}", "-".repeat(80));
    for (addr, balance) in results {
        let formatted_balance = format_units(balance, 6)
            .unwrap_or_else(|_| "Invalid".to_string());
        println!("📈 {} -> {} USDC", addr, formatted_balance);
    }

    Ok(())
}
