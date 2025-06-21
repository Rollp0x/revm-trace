//! # REVM-Trace Actix-Web Integration Example
//!
//! This example demonstrates how to integrate REVM-Trace with Actix-Web to create
//! a RESTful API for Ethereum transaction simulation and tracing.
//!
//! ## Features
//! - Transaction simulation with gas estimation
//! - Optional transaction tracing with call traces and asset transfers
//! - Support for both HTTP and WebSocket RPC endpoints
//! - Proper error handling and JSON responses
//! - Multi-threading support using `web::block` and `tokio::task::spawn_blocking`
//!
//! ## API Endpoints
//!
//! ### POST /simulate
//! Simulate a transaction using `tokio::task::spawn_blocking` approach.
//!
//! #### Request Body Example:
//! ```json
//! {
//!     "rpc_url": "https://eth.llamarpc.com",
//!     "from": "0xC255fC198eEdAC7AF8aF0f6e0ca781794B094A61",
//!     "to": "0xd878229c9c3575F224784DE610911B5607a3ad15",
//!     "value": "120000000000000000",
//!     "data": "0x",
//!     "with_trace": true
//! }
//! ```
//!
//! #### Response Example:
//! ```json
//! {
//!     "success": true,
//!     "gas_used": 21000,
//!     "error": null,
//!     "traces": {
//!         "asset_transfers": 1,
//!         "call_traces": {
//!             "from": "0xc255fc198eedac7af8af0f6e0ca781794b094a61",
//!             "to": "0xd878229c9c3575f224784de610911b5607a3ad15",
//!             "value": "0x1aa535d3d0c0000",
//!             "input": "0x",
//!             "call_scheme": "Call",
//!             "create_scheme": null,
//!             "gas_used": "0x0",
//!             "output": "0x",
//!             "status": "Success",
//!             "error_origin": false,
//!             "subtraces": [],
//!             "trace_address": []
//!         }
//!     }
//! }
//! ```
//!
//! ### POST /simulate_web_block
//! Simulate a transaction using `web::block` approach (recommended for actix-web).
//!
//! ### GET /health
//! Health check endpoint.
//!
//! ## Usage
//! ```bash
//! cargo run --example actix_web_integration
//! ```
//!
//! Then test with curl:
//! ```bash
//! curl -X POST http://127.0.0.1:8080/simulate \
//!   -H "Content-Type: application/json" \
//!   -d '{"rpc_url":"https://eth.llamarpc.com","from":"0xC255fC198eEdAC7AF8aF0f6e0ca781794B094A61","to":"0xd878229c9c3575F224784DE610911B5607a3ad15","value":"120000000000000000","with_trace":true}'
//! ```

use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware::Logger};
use revm_trace::{
    create_evm, create_evm_with_tracer, TxInspector,
    types::{SimulationTx, SimulationBatch},
    traits::TransactionTrace,
};
use serde::{Deserialize, Serialize};
use alloy::primitives::{U256, TxKind, Address};
use std::str::FromStr;

/// Request structure for transaction simulation
#[derive(Deserialize)]
struct SimulateRequest {
    /// RPC endpoint URL (HTTP or WebSocket)
    rpc_url: String,
    /// Transaction sender address (hex string with or without 0x prefix)
    from: String,
    /// Transaction recipient address (hex string with or without 0x prefix)
    to: String,
    /// Transaction value in wei (optional, defaults to 0)
    value: Option<String>,
    /// Transaction input data (optional, hex string with or without 0x prefix)
    data: Option<String>,
    /// Whether to include transaction traces (optional, defaults to false)
    with_trace: Option<bool>,
}

/// Response structure for transaction simulation
#[derive(Serialize)]
struct SimulateResponse {
    /// Whether the simulation was successful
    success: bool,
    /// Gas used by the transaction (if successful)
    gas_used: Option<u64>,
    /// Error message (if failed)
    error: Option<String>,
    /// Transaction traces (if with_trace=true and successful)
    traces: Option<serde_json::Value>,
}

/// Simulate transaction using tokio::task::spawn_blocking approach
/// 
/// This approach creates a new tokio runtime inside a blocking task to handle
/// the EVM operations that may block on network I/O.
async fn simulate_transaction(req: web::Json<SimulateRequest>) -> Result<HttpResponse> {
    let request = req.into_inner();
    
    // Use tokio::task::spawn_blocking to handle potentially blocking operations
    let result = tokio::task::spawn_blocking(move || {
        // Create a new runtime to handle EVM creation and execution
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            simulate_tx_internal(request).await
        })
    }).await;

    match result {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(SimulateResponse {
            success: false,
            gas_used: None,
            error: Some(format!("Runtime error: {}", e)),
            traces: None,
        })),
    }
}

/// Simulate transaction using web::block approach (recommended for actix-web)
/// 
/// This approach uses actix-web's built-in blocking thread pool, which is
/// more efficient and integrates better with the actix-web ecosystem.
async fn simulate_transaction_web_block(req: web::Json<SimulateRequest>) -> Result<HttpResponse> {
    let request = req.into_inner();
    
    // Use actix-web's web::block for better integration
    let result = web::block(move || {
        // Create a new runtime to handle EVM creation and execution
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            simulate_tx_internal(request).await
        })
    }).await;

    match result {
        Ok(response) => Ok(HttpResponse::Ok().json(response)),
        Err(e) => Ok(HttpResponse::InternalServerError().json(SimulateResponse {
            success: false,
            gas_used: None,
            error: Some(format!("Web block error: {}", e)),
            traces: None,
        })),
    }
}

/// Internal function to handle transaction simulation logic
/// 
/// This function contains the core simulation logic that is shared between
/// different endpoint implementations.
async fn simulate_tx_internal(request: SimulateRequest) -> SimulateResponse {
    // Parse addresses
    let from_addr = match Address::from_str(&request.from) {
        Ok(addr) => addr,
        Err(e) => return SimulateResponse {
            success: false,
            gas_used: None,
            error: Some(format!("Invalid from address: {}", e)),
            traces: None,
        },
    };

    let to_addr = match Address::from_str(&request.to) {
        Ok(addr) => addr,
        Err(e) => return SimulateResponse {
            success: false,
            gas_used: None,
            error: Some(format!("Invalid to address: {}", e)),
            traces: None,
        },
    };

    // Parse transaction value
    let value = if let Some(val_str) = request.value {
        match U256::from_str(&val_str) {
            Ok(val) => val,
            Err(e) => return SimulateResponse {
                success: false,
                gas_used: None,
                error: Some(format!("Invalid value: {}", e)),
                traces: None,
            },
        }
    } else {
        U256::ZERO
    };

    // Parse transaction data
    let data = if let Some(data_str) = request.data {
        match hex::decode(data_str.strip_prefix("0x").unwrap_or(&data_str)) {
            Ok(bytes) => bytes.into(),
            Err(e) => return SimulateResponse {
                success: false,
                gas_used: None,
                error: Some(format!("Invalid data: {}", e)),
                traces: None,
            },
        }
    } else {
        vec![].into()
    };

    // Create transaction object
    let tx = SimulationTx {
        caller: from_addr,
        transact_to: TxKind::Call(to_addr),
        value,
        data,
    };

    let batch = SimulationBatch {
        block_env: None,
        transactions: vec![tx],
        is_stateful: false,
    };

    // Choose EVM mode based on tracing requirement
    if request.with_trace.unwrap_or(false) {
        // Use tracing mode for detailed call traces and asset transfers
        match create_evm_with_tracer(&request.rpc_url, TxInspector::new()).await {
            Ok(mut evm) => {
                let results = evm.trace_transactions(batch);
                match results.into_iter().next() {
                    Some(Ok((execution_result, trace_output))) => {
                        SimulateResponse {
                            success: true,
                            gas_used: Some(execution_result.gas_used()),
                            error: None,
                            traces: Some(serde_json::json!({
                                "asset_transfers": trace_output.asset_transfers.len(),
                                "call_traces": trace_output.call_trace,
                            })),
                        }
                    }
                    Some(Err(e)) => SimulateResponse {
                        success: false,
                        gas_used: None,
                        error: Some(e.to_string()),
                        traces: None,
                    },
                    None => SimulateResponse {
                        success: false,
                        gas_used: None,
                        error: Some("No results returned".to_string()),
                        traces: None,
                    },
                }
            }
            Err(e) => SimulateResponse {
                success: false,
                gas_used: None,
                error: Some(format!("Failed to create tracing EVM: {}", e)),
                traces: None,
            },
        }
    } else {
        // Use standard mode for high performance (no tracing overhead)
        match create_evm(&request.rpc_url).await {
            Ok(mut evm) => {
                let results = evm.execute_batch(batch);
                match results.into_iter().next() {
                    Some(Ok(execution_result)) => {
                        SimulateResponse {
                            success: true,
                            gas_used: Some(execution_result.gas_used()),
                            error: None,
                            traces: None,
                        }
                    }
                    Some(Err(e)) => SimulateResponse {
                        success: false,
                        gas_used: None,
                        error: Some(e.to_string()),
                        traces: None,
                    },
                    None => SimulateResponse {
                        success: false,
                        gas_used: None,
                        error: Some("No results returned".to_string()),
                        traces: None,
                    },
                }
            }
            Err(e) => SimulateResponse {
                success: false,
                gas_used: None,
                error: Some(format!("Failed to create EVM: {}", e)),
                traces: None,
            },
        }
    }
}

/// Health check endpoint
/// 
/// Returns the service status and version information.
/// This endpoint can be used for monitoring and load balancer health checks.
async fn health_check() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "version": "3.0.0"
    })))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    println!("🚀 Starting REVM-Trace API Server...");
    println!("📡 Endpoints:");
    println!("   POST /simulate           - Simulate transactions (spawn_blocking)");
    println!("   POST /simulate_web_block - Simulate transactions (web::block)");
    println!("   GET  /health             - Health check");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/health", web::get().to(health_check))
            .route("/simulate", web::post().to(simulate_transaction))
            .route("/simulate_web_block", web::post().to(simulate_transaction_web_block))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
