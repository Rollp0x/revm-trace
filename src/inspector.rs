//! Transaction inspector for tracing EVM execution
//!
//! This module provides functionality for:
//! - Tracing contract calls and creations
//! - Tracking token transfers (both native and ERC20)
//! - Collecting execution logs
//! - Analyzing call hierarchies
//!
//! The inspector implements the revm Inspector trait to hook into
//! various points of EVM execution.

use alloy::primitives::{Address, Bytes, FixedBytes, Log, U256, hex, keccak256};
use revm::{
    Database, 
    EvmContext, 
    Inspector,
    interpreter::{
        CallInputs, 
        CallOutcome, 
        CreateInputs, 
        CreateOutcome,
        InstructionResult,
        Interpreter, 
        SuccessOrHalt,
    },
};
use once_cell::sync::Lazy;
use crate::{
    types::*,
    utils::error_utils::parse_custom_error,
};

// Event signature for ERC20 Transfer events
static TRANSFER_EVENT_SIGNATURE: Lazy<FixedBytes<32>> =
    Lazy::new(|| keccak256(b"Transfer(address,address,uint256)"));

/// Transaction inspector that collects execution traces and transfer events
#[derive(Default,Clone)]
pub struct TxInspector {
    /// Chronologically ordered list of all asset transfers
    transfers: Vec<TokenTransfer>,
    /// Complete list of all calls made during execution
    traces: Vec<CallTrace>,
    /// All emitted logs during transaction execution
    logs: Vec<Log>,
    /// Stack of trace indices to track call hierarchy
    call_stack: Vec<usize>,
    /// Stack of addresses to track the actual caller
    address_stack: Vec<Address>,
    /// Pending contract creation transfer that needs to be updated with the actual address
    pending_create_transfer: Option<(usize, TokenTransfer)>, // (index, transfer)
}

impl TxInspector {
    /// Creates a new inspector instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all recorded token transfers
    pub fn get_transfers(&self) -> &Vec<TokenTransfer> {
        &self.transfers
    }

    /// Returns all execution traces
    pub fn get_traces(&self) -> &Vec<CallTrace> {
        &self.traces
    }

    /// Returns all emitted logs
    pub fn get_logs(&self) -> &Vec<Log> {
        &self.logs
    }

    /// Resets the inspector state
    pub fn reset(&mut self) {
        self.traces = Vec::new();
        self.call_stack = Vec::new();
        self.transfers = Vec::new();
        self.logs = Vec::new();
        self.address_stack = Vec::new(); // 重置 address_stack
    }

    /// Handles the end of a call or create operation
    ///
    /// Updates the trace with execution results and maintains the call hierarchy
    ///
    /// # Arguments
    /// * `result` - Execution result status
    /// * `gas_used` - Amount of gas consumed
    /// * `output` - Output data from the call
    fn handle_end(&mut self, result: InstructionResult, gas_used: u64, output: Bytes) {
        if let Some(trace_index) = self.call_stack.pop() {
            let trace = &mut self.traces[trace_index];
            trace.gas_used = U256::from(gas_used);
            trace.output = output.clone();

            // Convert execution result to call status
            let status = match SuccessOrHalt::from(result) {
                SuccessOrHalt::Success(_) => CallStatus::Success,
                SuccessOrHalt::Revert => {
                    if let Some(error_msg) = parse_custom_error(&output) {
                        CallStatus::Revert(error_msg)
                    } else {
                        CallStatus::Revert(format!("0x{}", hex::encode(output)))
                    }
                },
                SuccessOrHalt::Halt(reason) => CallStatus::Halt(format!("{:?}", reason)),
                SuccessOrHalt::FatalExternalError => CallStatus::FatalError,
                SuccessOrHalt::Internal(_) => CallStatus::Success,
            };

            trace.status = status.clone();
            
            // Mark as error origin if this call failed but all subtraces succeeded
            trace.error_origin = !trace.status.is_success() && 
                trace.subtraces.iter().all(|subtrace| subtrace.status.is_success());

            // Move trace to parent's subtraces if there is a parent call
            if let Some(&parent_index) = self.call_stack.last() {
                let trace = self.traces.remove(trace_index);
                self.traces[parent_index].subtraces.push(trace);
            }
        }
    }
}

/// Parses a Transfer event log into its components
///
/// # Arguments
/// * `topics` - Event topics array containing:
///   - [0]: Event signature (keccak256 of "Transfer(address,address,uint256)")
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
    let amount = U256::from_be_slice(data);
    if !amount.is_zero() {
        Some((
            Address::from_slice(&topics[1].as_slice()[12..]),
            Address::from_slice(&topics[2].as_slice()[12..]),
            amount,
        ))
    } else {
        None
    }
}

impl<DB: Database> Inspector<DB> for TxInspector {
    /// Handles contract calls
    ///
    /// Records ETH transfers and creates a new trace entry
    fn call(
        &mut self,
        _context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        let from = if let Some(&last_address) = self.address_stack.last() {
            last_address
        } else {
            inputs.caller
        };

        let to = match inputs.scheme {
            CallScheme::DelegateCall => inputs.bytecode_address,
            _ => inputs.target_address,
        };


        // Record ETH transfers
        if let Some(value) = inputs.transfer_value() {
            if value > U256::ZERO && (inputs.scheme == CallScheme::Call || inputs.scheme == CallScheme::CallCode) {
                self.transfers.push(TokenTransfer {
                    token: NATIVE_TOKEN_ADDRESS,
                    from: inputs.transfer_from(),
                    to: Some(inputs.transfer_to()),
                    value,
                });
            }
        }

        let next_caller = match inputs.scheme {
            CallScheme::DelegateCall => from,
            _ => to,
        };
        self.address_stack.push(next_caller);

        let trace = CallTrace {
            from,
            to,
            value: inputs.call_value(),
            input: inputs.input.clone(),
            call_scheme: Some(inputs.scheme),
            create_scheme: None,
            gas_used: U256::ZERO,
            output: Bytes::new(),
            status: CallStatus::InProgress,
            error_origin: false,
            subtraces: Vec::new(),
            trace_address: Vec::new(),
        };

        self.traces.push(trace);
        self.call_stack.push(self.traces.len() - 1);
        None
    }

    /// Handles contract creation
    ///
    /// Creates a new trace entry for the contract creation
    fn create(
        &mut self,
        _context: &mut EvmContext<DB>,
        inputs: &mut CreateInputs,
    ) -> Option<CreateOutcome> {
        let from = inputs.caller;
        self.address_stack.push(from);

        // Record ETH transfer for contract creation
        if inputs.value > U256::ZERO {
            let transfer = TokenTransfer {
                token: NATIVE_TOKEN_ADDRESS,
                from,
                to:None,  // Will be updated in create_end
                value: inputs.value,
            };
            self.transfers.push(transfer.clone());
            self.pending_create_transfer = Some((self.transfers.len() - 1, transfer));
        }

        // Create new trace entry
        let mut trace_address = Vec::new();
        if !self.call_stack.is_empty() {
            let parent_index = *self.call_stack.last().unwrap();
            trace_address = self.traces[parent_index].trace_address.clone();
            trace_address.push(self.traces[parent_index].subtraces.len());
        }

        let trace = CallTrace {
            from,
            to: Address::ZERO,  // Will be updated in create_end
            value: inputs.value,
            input: inputs.init_code.clone(),
            call_scheme: None,
            create_scheme: Some(inputs.scheme),
            gas_used: U256::ZERO,
            output: Bytes::new(),
            status: CallStatus::InProgress,  // Will be updated in create_end
            error_origin: false,
            subtraces: Vec::new(),
            trace_address,
        };

        self.traces.push(trace);
        self.call_stack.push(self.traces.len() - 1);
        None
    }

    /// Handles the end of a contract call
    fn call_end(
        &mut self,
        _context: &mut EvmContext<DB>,
        inputs: &CallInputs,
        outcome: CallOutcome,
    ) -> CallOutcome {
        self.handle_end(outcome.result.result, outcome.result.gas.spent(), outcome.result.output.clone());
        match inputs.scheme {
            CallScheme::DelegateCall => {}
            _ => {
                self.address_stack.pop();
            }
        }
        outcome
    }

    /// Handles the end of a contract creation
    fn create_end(
        &mut self,
        _context: &mut EvmContext<DB>,
        _inputs: &CreateInputs,
        outcome: CreateOutcome,
    ) -> CreateOutcome {
        if let Some(address) = outcome.address {
            // Update the trace
            if let Some(trace_index) = self.call_stack.last() {
                self.traces[*trace_index].to = address;
            }
            
            // Update the transfer
            if let Some((transfer_index, mut transfer)) = self.pending_create_transfer.take() {
                transfer.to = Some(address);
                self.transfers[transfer_index] = transfer;
            }
        }
        self.handle_end(outcome.result.result, outcome.result.gas.spent(), outcome.result.output.clone());
        self.address_stack.pop();
        outcome
    }

    /// Monitors and records ERC20 token transfers
    ///
    /// Stores all logs and parses Transfer events to track token movements
    fn log(&mut self, _interp: &mut Interpreter, _context: &mut EvmContext<DB>, log: &Log) {
        // Store all logs
        self.logs.push(log.clone());
        
        // Parse and record Transfer events
        if let Some((from, to, amount)) = parse_transfer_log(log.topics(), &log.data.data) {
            self.transfers.push(TokenTransfer {
                token: log.address,
                from,
                to: Some(to),
                value: amount,
            });
        }
    }

    /// Handles contract self-destruction
    ///
    /// Records the transfer of remaining ETH balance
    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        if value > U256::ZERO {
            self.transfers.push(TokenTransfer {
                token: NATIVE_TOKEN_ADDRESS,
                from: contract,      // Address of the self-destructed contract
                to: Some(target),          // Address receiving the balance
                value,              // Remaining contract balance
            });
        }
    }
}
