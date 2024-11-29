//! Transaction execution inspector and trace collection
//!
//! This module provides the `TransactionTracer` inspector implementation for:
//! - Tracking ETH and token transfers
//! - Recording detailed call traces
//! - Collecting all transaction logs
//! - Parsing custom error messages
//! - Formatting execution traces for analysis
//!
//! # Features
//! - **Asset Tracking**: Monitors both native token and ERC20 token transfers
//! - **Call Tracing**: Records complete call hierarchy with gas usage
//! - **Log Collection**: Captures all emitted logs during transaction execution
//! - **Error Handling**: Parses both EVM and custom Solidity error messages
//! - **Detailed Traces**: Provides formatted output for debugging and analysis
//!
//! # Example
//! ```no_run
//! use revm_trace::{create_evm_instance_with_tracer, trace_tx_assets};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut evm = create_evm_instance_with_tracer(
//!     "https://eth-mainnet.example.com",
//!     Some(1)
//! )?;
//!
//! let result = trace_tx_assets(/* ... */).await?;
//!
//! // Access transaction logs
//! for log in result.inspector.get_logs() {
//!     println!("Log from {}: {:?}", log.address, log);
//! }
//!
//! // Check for errors
//! if let Some(error_trace) = result.inspector.find_error_trace() {
//!     if let Some(error_msg) = error_trace.format_error() {
//!         println!("Transaction failed: {}", error_msg);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use super::types::{CallTrace, ExecutionError, TransferRecord};
use alloy::primitives::{hex, keccak256, Address, FixedBytes, Log, U256};
use once_cell::sync::Lazy;
use revm::interpreter::{CallInputs, CallOutcome, CallScheme, InstructionResult, Interpreter};
use revm::{Database, EvmContext, Inspector};
use std::fmt;

/// Transfer event signature: keccak256("Transfer(address,address,uint256)")
///
/// Used to identify ERC20 Transfer events in transaction logs
static TRANSFER_EVENT_SIGNATURE: Lazy<FixedBytes<32>> =
    Lazy::new(|| keccak256(b"Transfer(address,address,uint256)"));

/// Parses a Transfer event log into its components
///
/// # Arguments
/// * `topics` - Event topics array containing:
///   - [0]: Event signature (keccak256 of the event name)
///   - [1]: From address (indexed)
///   - [2]: To address (indexed)
/// * `data` - ABI-encoded amount parameter
///
/// # Returns
/// * `Some((from, to, amount))` - If the log is a valid Transfer event
/// * `None` - If the log format is invalid or not a Transfer event
fn parse_transfer_log(topics: &[FixedBytes<32>], data: &[u8]) -> Option<(Address, Address, U256)> {
    if topics.len() < 3 || topics[0] != *TRANSFER_EVENT_SIGNATURE {
        return None;
    }

    Some((
        Address::from_slice(&topics[1].as_slice()[12..]),
        Address::from_slice(&topics[2].as_slice()[12..]),
        U256::from_be_slice(data),
    ))
}

/// Display implementation for execution errors
impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionError::Evm(result) => write!(f, "EVM Error: {:?}", result),
            ExecutionError::Custom(msg) => write!(f, "Contract Error: {}", msg),
        }
    }
}

/// Inspector for comprehensive transaction execution tracing
#[derive(Default)]
pub struct TransactionTracer {
    /// Chronologically ordered list of all asset transfers
    pub transfers: Vec<TransferRecord>,
    /// Current call stack for tracking nested calls
    pub call_stack: Vec<CallTrace>,
    /// Complete list of all calls made during execution
    pub traces: Vec<CallTrace>,
    /// All emitted logs during transaction execution
    pub logs: Vec<Log>,
}

impl TransactionTracer {
    /// Creates a new transaction tracer instance
    pub fn new() -> Self {
        Self {
            transfers: Vec::new(),
            call_stack: Vec::new(),
            traces: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Locates the specific call trace where an error occurred
    ///
    /// Recursively searches through the call tree to find the trace
    /// marked as the error origin.
    pub fn find_error_trace(&self) -> Option<&CallTrace> {
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

    /// Attempts to parse a custom error message from revert data
    ///
    /// Handles the standard Solidity error format:
    /// - Selector: 0x08c379a0
    /// - Data: ABI-encoded string
    ///
    /// # Arguments
    /// * `output` - Raw output data from the reverted call
    ///
    /// # Returns
    /// * `Some(String)` - Parsed error message
    /// * `None` - If the data isn't a custom error or is malformed
    fn parse_custom_error(output: &[u8]) -> Option<String> {
        // Custom error format: 0x08c379a0 + (string length + string data in ABI encoding)
        if output.len() >= 4 && output.starts_with(b"\x08\xc3\x79\xa0") {
            // Skip selector (4 bytes) and offset (32 bytes)
            let data = &output[36..];
            if data.len() >= 32 {
                // Read string length
                let length = U256::from_be_slice(&data[..32]);
                let length = length.to_string().parse::<usize>().unwrap();
                // Read string data
                if data.len() >= 32 + length {
                    if let Ok(error_msg) = String::from_utf8(data[32..32 + length].to_vec()) {
                        return Some(error_msg);
                    }
                }
            }
        }
        None
    }

    /// Returns all collected logs from the transaction
    pub fn get_logs(&self) -> &[Log] {
        &self.logs
    }
}

impl<DB: Database> Inspector<DB> for TransactionTracer {
    /// Captures call information and native token transfers
    ///
    /// Records:
    /// - Call context (from, to, value, input)
    /// - Native token transfers for CALL and CALLCODE
    /// - Updates call stack and trace hierarchy
    fn call(
        &mut self,
        _context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        // Track native token transfers - only for regular CALL and CALLCODE
        if let Some(value) = inputs.transfer_value() {
            // Only track transfers for CALL and CALLCODE
            match inputs.scheme {
                CallScheme::Call | CallScheme::CallCode => {
                    if value > U256::ZERO {
                        self.transfers.push(TransferRecord::new_native_token(
                            inputs.transfer_from(),
                            inputs.transfer_to(),
                            value,
                        ));
                    }
                }
                // DELEGATECALL, STATICCALL
                CallScheme::DelegateCall
                | CallScheme::StaticCall
                | CallScheme::ExtCall
                | CallScheme::ExtStaticCall
                | CallScheme::ExtDelegateCall => {}
            }
        }

        // Create new call trace
        let trace = CallTrace {
            from: inputs.caller,
            scheme: inputs.scheme,
            gas_limit: inputs.gas_limit,
            input: inputs.input.clone(),
            to: inputs.target_address,
            value: inputs.transfer_value(),
            result: None,
            gas: None,
            output: None,
            subtraces: Vec::new(),
            trace_address: if self.call_stack.is_empty() {
                Vec::new()
            } else {
                let mut addr = self.call_stack.last().unwrap().trace_address.clone();
                addr.push(self.call_stack.last().unwrap().subtraces.len());
                addr
            },
            error: None,
            error_origin: false,
        };

        // Push to call stack
        self.call_stack.push(trace);

        None
    }

    /// Processes call results and updates trace information
    ///
    /// Updates the trace with:
    /// - Execution result
    /// - Gas usage
    /// - Output data
    /// - Error information (if any)
    fn call_end(
        &mut self,
        _context: &mut EvmContext<DB>,
        _inputs: &CallInputs,
        outcome: CallOutcome,
    ) -> CallOutcome {
        if let Some(mut trace) = self.call_stack.pop() {
            // Update trace with results
            trace.result = Some(outcome.result.result);
            trace.gas = Some(outcome.result.gas);
            trace.output = Some(outcome.result.output.clone());
            // Check for errors
            if outcome.result.result == InstructionResult::Revert {
                // Check for custom error
                if let Some(error_msg) = Self::parse_custom_error(&outcome.result.output) {
                    trace.error = Some(ExecutionError::Custom(error_msg));
                } else {
                    trace.error = Some(ExecutionError::Evm(outcome.result.result));
                }
                trace.error_origin = true;
            }

            // Add to parent's subtraces or main traces list
            if let Some(parent) = self.call_stack.last_mut() {
                parent.subtraces.push(trace);
            } else {
                self.traces.push(trace);
            }
        }
        outcome
    }

    /// Monitors and records ERC20 token transfers
    ///
    /// Parses Transfer events to track token movements between addresses
    fn log(&mut self, _interp: &mut Interpreter, _context: &mut EvmContext<DB>, log: &Log) {
        // Store all logs
        self.logs.push(log.clone());
        
        // Continue processing Transfer events
        if let Some((from, to, amount)) = parse_transfer_log(log.topics(), &log.data.data) {
            self.transfers
                .push(TransferRecord::new_token(log.address, from, to, amount));
        }
    }
}

impl TransactionTracer {
    /// Returns all collected call traces
    pub fn get_traces(&self) -> &[CallTrace] {
        &self.traces
    }

    /// Formats all traces for human-readable display
    ///
    /// Generates a detailed text representation of the entire call tree
    pub fn format_traces(&self) -> String {
        let mut output = String::new();
        for (i, trace) in self.traces.iter().enumerate() {
            output.push_str(&format!("Call {}: {}\n", i, self.format_trace(trace, 0)));
        }
        output
    }

    /// Formats a single call trace with proper indentation
    ///
    /// Includes:
    /// - Call context (addresses, scheme, value)
    /// - Input data (hex encoded)
    /// - Execution result and gas usage
    /// - Output data (if any)
    /// - Nested calls (recursively formatted)
    #[allow(clippy::only_used_in_recursion)]
    fn format_trace(&self, trace: &CallTrace, depth: usize) -> String {
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
}

impl CallTrace {
    /// Formats error information for a failed call
    ///
    /// Generates a detailed error message including:
    /// - Call context (addresses, scheme)
    /// - Transfer value (if any)
    /// - Input data
    /// - Error description
    pub fn format_error(&self) -> Option<String> {
        self.error.as_ref().map(|err| {
            let mut msg = format!("Error in call to {} [{:?}]:\n", self.to, self.scheme);
            msg.push_str(&format!("  From: {}\n", self.from));
            if let Some(value) = &self.value {
                msg.push_str(&format!("  Value: {} wei\n", value));
            }
            if !self.input.is_empty() {
                msg.push_str(&format!("  Input: 0x{}\n", hex::encode(&self.input)));
            }
            msg.push_str(&format!("  Error: {}\n", err));
            msg
        })
    }
}
