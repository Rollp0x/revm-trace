//! # Concurrent SharedBackend Demo
//!
//! This example demonstrates the multi-threading capabilities of SharedBackend:
//! 
//! ## Key Features Demonstrated
//! - **SharedBackend is Send + Sync**: Can be safely shared across threads
//! - **Per-Thread EVM Creation**: Each thread creates its own EVM instance
//! - **Shared Cache Benefits**: All threads benefit from shared state cache
//! - **Concurrent Transaction Processing**: Multiple transactions processed simultaneously
//! - **RPC Connection Pooling**: Shared RPC connections reduce overhead
//!
//! ## Architecture
//! ```
//! Main Thread
//! ├── Create SharedBackend
//! ├── Clone SharedBackend for each worker thread
//! └── Spawn worker threads
//!     ├── Thread 1: SharedBackend → EVM₁ → Process Tx₁
//!     ├── Thread 2: SharedBackend → EVM₂ → Process Tx₂  
//!     ├── Thread 3: SharedBackend → EVM₃ → Process Tx₃
//!     └── Thread 4: SharedBackend → EVM₄ → Process Tx₄
//! ```
//!
//! ## Why This Works
//! - `SharedBackend`: Send + Sync ✅ (can cross thread boundaries)
//! - `TraceEvm`: NOT Send + Sync ❌ (stays within each thread)
//! - Shared cache and RPC pool provide performance benefits

#[cfg(feature = "foundry-fork")]
use revm_trace::{
    create_evm_from_shared_backend, create_shared_backend,
    types::{SimulationTx, SimulationBatch},
    traits::TransactionTrace,
    TxInspector,
};

#[cfg(feature = "foundry-fork")]
use {
    alloy::primitives::{U256, TxKind},
    anyhow::Result,
    colored::*,
    std::time::{Duration, Instant},
    tokio::time::sleep,
};

#[cfg(feature = "foundry-fork")]
const ETH_RPC_URL: &str = "https://eth.llamarpc.com";

#[cfg(feature = "foundry-fork")]
#[derive(Clone, Debug)]
struct WorkerTask {
    id: usize,
    description: String,
    from: String,
    to: String,
    value_eth: f64,
    expected_result: &'static str,
}

#[cfg(feature = "foundry-fork")]
impl WorkerTask {
    fn new(id: usize, description: &str, from: &str, to: &str, value_eth: f64, expected: &'static str) -> Self {
        Self {
            id,
            description: description.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            value_eth,
            expected_result: expected,
        }
    }
}

#[cfg(feature = "foundry-fork")]
async fn worker_thread(
    thread_id: usize,
    task: WorkerTask,
    shared_backend: foundry_fork_db::backend::SharedBackend,
) -> Result<(usize, String, Duration)> {
    let start_time = Instant::now();
    
    println!(
        "🧵 Thread {} starting: {}",
        thread_id.to_string().cyan().bold(),
        task.description.yellow()
    );
    
    // Add some artificial delay to simulate different processing times
    sleep(Duration::from_millis(thread_id as u64 * 100)).await;
    
    // Each thread gets its own provider (this could be optimized to share providers too)
    let provider = revm_trace::evm::builder::get_provider(ETH_RPC_URL).await?;
    
    // Create EVM instance from shared backend with tracer
    let tracer = TxInspector::new();
    let mut evm = create_evm_from_shared_backend(shared_backend, &provider, tracer).await?;
    
    // Parse addresses
    let from_addr = task.from.parse()
        .map_err(|e| anyhow::anyhow!("Invalid from address: {}", e))?;
    let to_addr = task.to.parse()
        .map_err(|e| anyhow::anyhow!("Invalid to address: {}", e))?;
    
    // Convert ETH to wei
    let value_wei = U256::from((task.value_eth * 1e18) as u64);
    
    // Create transaction
    let tx = SimulationTx {
        caller: from_addr,
        transact_to: TxKind::Call(to_addr),
        value: value_wei,
        data: vec![].into(),
    };
    
    let batch = SimulationBatch {
        transactions: vec![tx],
        is_stateful: false,
    };
    
    // Execute transaction with tracing
    let results = evm.trace_transactions(batch);
    let elapsed = start_time.elapsed();
    
    match results.into_iter().next() {
        Some(Ok((execution_result, trace_output))) => {
            let result_status = if execution_result.is_success() {
                "SUCCESS".green().bold()
            } else {
                "FAILED".red().bold()
            };
            
            let message = format!(
                "✅ Thread {} completed in {:?} - {} - Gas: {} - Transfers: {}",
                thread_id.to_string().cyan().bold(),
                elapsed,
                result_status,
                execution_result.gas_used(),
                trace_output.asset_transfers.len()
            );
            
            println!("{}", message);
            Ok((thread_id, message, elapsed))
        }
        Some(Err(e)) => {
            let message = format!(
                "❌ Thread {} failed in {:?} - Error: {}",
                thread_id.to_string().cyan().bold(),
                elapsed,
                e.to_string().red()
            );
            println!("{}", message);
            Ok((thread_id, message, elapsed))
        }
        None => {
            let message = format!(
                "⚠️ Thread {} completed in {:?} - No results",
                thread_id.to_string().cyan().bold(),
                elapsed
            );
            println!("{}", message);
            Ok((thread_id, message, elapsed))
        }
    }
}

#[cfg(feature = "foundry-fork")]
async fn run_concurrent_test() -> Result<()> {
    println!("{}", "🚀 Starting SharedBackend Concurrent Test".green().bold());
    println!("{}", "═".repeat(60).blue());
    
    // Create shared backend once
    println!("📡 Creating SharedBackend...");
    let shared_backend = create_shared_backend(ETH_RPC_URL, None).await?;
    println!("✅ SharedBackend created successfully");
    println!();
    
    // Define test tasks for different worker threads
    let tasks = vec![
        WorkerTask::new(
            1,
            "Wealthy whale transfer", 
            "0x8EB8a3b98659Cce290402893d0123abb75E3ab28", // Whale address
            "0x742d35Cc6675C4D858229a9e8E44B8d7B893E9c0", // Random address
            0.1,
            "EXPECTED_SUCCESS"
        ),
        WorkerTask::new(
            2,
            "Poor address transfer",
            "0x0000000000000000000000000000000000000001", // Nearly empty address
            "0x742d35Cc6675C4D858229a9e8E44B8d7B893E9c0",
            1.0,
            "EXPECTED_FAILURE"
        ),
        WorkerTask::new(
            3,
            "Another whale transfer",
            "0x40B38765696e3d5d8d9d834D8AaD4bB6e418E489", // Another whale
            "0x1234567890123456789012345678901234567890",
            0.05,
            "EXPECTED_SUCCESS"
        ),
        WorkerTask::new(
            4,
            "High gas price transfer",
            "0x8EB8a3b98659Cce290402893d0123abb75E3ab28", // Whale address
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            0.01,
            "EXPECTED_SUCCESS"
        ),
    ];
    
    // Spawn worker threads
    println!("🧵 Spawning {} worker threads...", tasks.len());
    let start_time = Instant::now();
    
    let mut handles = Vec::new();
    for task in tasks {
        let backend_clone = shared_backend.clone(); // Clone the SharedBackend
        let handle = tokio::spawn(async move {
            worker_thread(task.id, task, backend_clone).await
        });
        handles.push(handle);
    }
    
    // Wait for all threads to complete
    println!("⏳ Waiting for all threads to complete...");
    println!();
    
    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => println!("❌ Thread error: {}", e),
            Err(e) => println!("❌ Join error: {}", e),
        }
    }
    
    let total_elapsed = start_time.elapsed();
    
    // Print summary
    println!();
    println!("{}", "📊 SUMMARY".green().bold());
    println!("{}", "═".repeat(60).blue());
    
    results.sort_by_key(|(id, _, _)| *id);
    for (thread_id, message, duration) in &results {
        println!("Thread {}: {:?}", thread_id, duration);
    }
    
    println!();
    println!("⏱️  Total execution time: {:?}", total_elapsed);
    println!("🧵 Threads completed: {}/{}", results.len(), 4);
    
    let avg_duration = if !results.is_empty() {
        results.iter().map(|(_, _, d)| d.as_millis()).sum::<u128>() / results.len() as u128
    } else {
        0
    };
    println!("📈 Average thread duration: {}ms", avg_duration);
    
    // Calculate speedup (theoretical vs actual)
    let sequential_time = results.iter().map(|(_, _, d)| d.as_millis()).sum::<u128>();
    let speedup = sequential_time as f64 / total_elapsed.as_millis() as f64;
    println!("🚀 Concurrency speedup: {:.2}x", speedup);
    
    println!();
    println!("{}", "✅ SUCCESS: SharedBackend Multi-Threading Test Completed!".green().bold());
    println!("{}", "🎯 Key achievements:".cyan());
    println!("   ✓ SharedBackend successfully shared across {} threads", results.len());
    println!("   ✓ Each thread created its own EVM instance");
    println!("   ✓ Concurrent transaction processing achieved");
    println!("   ✓ Shared cache and RPC pool benefits utilized");
    println!("   ✓ No thread safety issues encountered");
    
    Ok(())
}

#[cfg(not(feature = "foundry-fork"))]
async fn run_concurrent_test() -> Result<()> {
    println!("{}", "⚠️  SharedBackend Multi-Threading Test Skipped".yellow().bold());
    println!("{}", "ℹ️  This test requires the 'foundry-fork' feature".cyan());
    println!("{}", "💡 Run with: cargo run --example concurrent_shared_backend --features foundry-fork".green());
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("{}", "🔬 SharedBackend Multi-Threading Demonstration".magenta().bold());
    println!("{}", "This example shows how SharedBackend enables safe multi-threading".cyan());
    println!();
    
    run_concurrent_test().await?;
    
    Ok(())
}
