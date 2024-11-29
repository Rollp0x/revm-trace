//! Types and structures for transaction tracing and analysis
//!
//! This module defines the core data structures used for:
//! - Asset transfer tracking (native token and ERC20 tokens)
//! - Transaction logs and events collection
//! - Call trace recording and hierarchy
//! - Log and event analysis
//! - Error handling and reporting
//! - Result formatting and analysis
//!
//! # Key Components
//! - `TraceResult`: Comprehensive transaction execution results including logs
//! - `TransferRecord`: Individual token transfer events
//! - `CallTrace`: Detailed call execution information
//! - `TokenInfo`: Token metadata and formatting
//! - `ExecutionError`: Structured error reporting
//!
//! All types implement `Serialize` and `Deserialize` for JSON compatibility.

use alloy::primitives::{address, hex, Address, Bytes, U256,Log};
use revm::interpreter::{CallScheme, Gas, InstructionResult};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Represents an error that occurred during transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionError {
    /// EVM-level errors (e.g., out of gas, stack overflow, invalid opcode)
    Evm(InstructionResult),
    /// Contract-specific custom errors (e.g., "insufficient balance", "invalid k")
    Custom(String),
}

/// Represents a single asset transfer event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Token contract address (NATIVE_TOKEN_ADDRESS for native token transfers)
    pub token: Address,
    /// Sender address
    pub from: Address,
    /// Recipient address
    pub to: Address,
    /// Transfer amount in the token's smallest unit
    pub value: U256,
}

impl TransferRecord {
    /// Creates a new native token transfer record
    pub fn new_native_token(from: Address, to: Address, value: U256) -> Self {
        Self {
            token: NATIVE_TOKEN_ADDRESS,
            from,
            to,
            value,
        }
    }

    /// Creates a new token transfer record
    pub fn new_token(token: Address, from: Address, to: Address, value: U256) -> Self {
        Self {
            token,
            from,
            to,
            value,
        }
    }

    /// Returns true if this represents an native token transfer
    pub fn is_native_token(&self) -> bool {
        self.token == NATIVE_TOKEN_ADDRESS
    }
}

/// Token native_token_symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token symbol (e.g., "USDT", "DAI", "ETH)
    pub symbol: String,
    /// Number of decimal places (e.g., 18 for ETH, 6 for USDT)
    pub decimals: u8,
}

/// Special zero address used to represent native token (ETH/BNB/MATIC/etc)
pub const NATIVE_TOKEN_ADDRESS: Address = address!("0000000000000000000000000000000000000000");

/// Comprehensive result of transaction trace analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct TraceResult {
    /// Chronologically ordered list of all asset transfers
    pub asset_transfers: Vec<TransferRecord>,
    /// Map of token addresses to their metadata
    pub token_info: HashMap<Address, TokenInfo>,
    /// Complete hierarchy of calls made during execution
    pub traces: Vec<CallTrace>,
    /// All emitted logs during transaction execution
    pub logs: Vec<Log>,
    /// Error information if the transaction failed
    pub error: Option<ExecutionError>,
}

impl TraceResult {
    /// Creates a new trace result with the given data
    ///
    /// # Arguments
    /// * `transfers` - List of all token transfers
    /// * `token_info` - Token metadata map
    /// * `traces` - Call execution traces
    /// * `logs` - Transaction event logs
    /// * `raw_token_symbol` - Native token symbol (e.g., "ETH", "BNB")
    ///
    /// # Features
    /// - Automatically adds native token info
    /// - Extracts error information from traces
    /// - Preserves chronological order of transfers
    /// - Maintains complete log history
    pub fn new(
        transfers: Vec<TransferRecord>,
        mut token_info: HashMap<Address, TokenInfo>,
        traces: Vec<CallTrace>,
        logs: Vec<Log>,
        raw_token_symbol: &str,
    ) -> Self {
        // Add native tokenInfo
        token_info.insert(
            NATIVE_TOKEN_ADDRESS,
            TokenInfo {
                symbol: raw_token_symbol.to_string(),
                decimals: 18,
            },
        );

        // Extract error from traces if present
        let error = traces
            .iter()
            .find(|trace| trace.error_origin)
            .and_then(|trace| trace.error.clone());

        Self {
            asset_transfers: transfers,
            token_info,
            traces,
            logs,
            error,
        }
    }

    /// Get all asset transfers (both native token and ERC20)
    pub fn asset_transfers(&self) -> &[TransferRecord] {
        &self.asset_transfers
    }

    /// Returns the specific call trace where an error originated
    ///
    /// Recursively searches through the call tree to find the trace
    /// marked as the error origin.
    pub fn error_trace(&self) -> Option<&CallTrace> {
        fn find_error_origin(traces: &[CallTrace]) -> Option<&CallTrace> {
            for trace in traces {
                if trace.error_origin {
                    return Some(trace);
                }
                if let Some(error_trace) = find_error_origin(&trace.subtraces) {
                    return Some(error_trace);
                }
            }
            None
        }
        find_error_origin(&self.traces)
    }

    /// Formats all call traces for human-readable display
    ///
    /// Returns a string containing the complete call hierarchy with:
    /// - Call addresses and types
    /// - Input and output data
    /// - Gas usage
    /// - Error information
    #[allow(clippy::only_used_in_recursion)]
    pub fn format_traces(&self) -> String {
        let mut output = String::new();
        for (i, trace) in self.traces.iter().enumerate() {
            output.push_str(&format!("Call {}: {}\n", i, self.format_trace(trace, 0)));
        }
        output
    }

    /// Formats a single call trace with proper indentation
    #[allow(clippy::only_used_in_recursion)]
    fn format_trace(&self, trace: &CallTrace, depth: usize) -> String {
        // ... implementation ...
        let indent = "  ".repeat(depth);
        let mut output = format!(
            "{}[{}] {} -> {} [{:?}] value: {:?}\n",
            indent,
            trace
                .trace_address
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join("/"),
            trace.from,
            trace.to,
            trace.scheme,
            trace.value,
        );

        if !trace.input.is_empty() {
            output.push_str(&format!("{}Input: {}\n", indent, hex::encode(&trace.input)));
        }

        if let Some(result) = &trace.result {
            output.push_str(&format!("{}Result: {:?}\n", indent, result));
        }

        if let Some(gas) = &trace.gas {
            output.push_str(&format!("{}Gas used: {:?}\n", indent, gas));
        }

        if let Some(output_data) = &trace.output {
            if !output_data.is_empty() {
                output.push_str(&format!("{}Output: {}\n", indent, hex::encode(output_data)));
            }
        }

        for subtrace in &trace.subtraces {
            output.push_str(&self.format_trace(subtrace, depth + 1));
        }

        output
    }

    /// Returns a formatted error message if the transaction failed
    pub fn format_error(&self) -> Option<String> {
        self.error_trace().and_then(|trace| trace.format_error())
    }
}

/// Represents a single call in the transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTrace {
    /// Address that initiated the call
    pub from: Address,
    /// Call type (CALL, STATICCALL, DELEGATECALL, etc)
    pub scheme: CallScheme,
    /// Maximum gas allocated for this call
    pub gas_limit: u64,
    /// Call input data (function selector and parameters)
    pub input: Bytes,
    /// Target contract address
    pub to: Address,
    /// Native token value transferred with the call (in wei)
    pub value: Option<U256>,
    /// Execution result (Success, Revert, etc)
    pub result: Option<InstructionResult>,
    /// Gas usage details
    pub gas: Option<Gas>,
    /// Call output data (return values or revert data)
    pub output: Option<Bytes>,
    /// Nested calls made during execution
    pub subtraces: Vec<CallTrace>,
    /// Call position in execution tree (e.g., [0,1] = second subcall of first call)
    pub trace_address: Vec<usize>,
    /// Error details if call failed
    pub error: Option<ExecutionError>,
    /// Indicates if this call originated an error
    pub error_origin: bool,
}
