//! REVM Inspector implementation for transaction tracing
//!
//! This module implements the core REVM Inspector hooks to track:
//! - Contract calls and creations
//! - Asset transfers (ETH and ERC20)
//! - Event logs and state changes
//! - Call hierarchy and execution flow
//!
//! The implementation carefully handles special cases like:
//! - Delegate calls and their caller context
//! - Contract creation address resolution
//! - Self-destructs and balance transfers
//! - ERC20 transfer event parsing

use crate::TxInspector;
use revm::{
    context::ContextTr,
    interpreter::{
        CallInputs, CallOutcome, CallScheme, CreateInputs, CreateOutcome, Interpreter,
        InterpreterTypes,
    },
    Inspector,
};

use crate::types::*;
use alloy::primitives::{Address, Bytes, Log, U256};

impl<CTX, INTR> Inspector<CTX, INTR> for TxInspector
where
    CTX: ContextTr,
    INTR: InterpreterTypes,
{
    /// Handles contract calls (both regular and delegate)
    ///
    /// # Processing Steps
    /// 1. Determines effective caller based on call context
    /// 2. Records any ETH transfers
    /// 3. Updates address stack for delegate calls
    /// 4. Creates and stores call trace entry
    ///
    /// # Special Cases
    /// - Delegate calls: Maintains original caller
    /// - Value transfers: Only tracked for regular calls
    ///
    fn call(&mut self, context: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        let mut from = self.address_stack.last().copied().unwrap_or(inputs.caller);
        if from == Address::ZERO {
            // If from is zero, use the caller as the effective from address
            from = inputs.caller;
        }
        let to = match inputs.scheme {
            CallScheme::DelegateCall => inputs.bytecode_address,
            _ => inputs.target_address,
        };

        // Track ETH transfers
        if let Some(value) = inputs.transfer_value() {
            if value > U256::ZERO
                && (inputs.scheme == CallScheme::Call || inputs.scheme == CallScheme::CallCode)
            {
                self.transfers.push(TokenTransfer {
                    token: NATIVE_TOKEN_ADDRESS,
                    from: inputs.transfer_from(),
                    to: Some(inputs.transfer_to()),
                    token_type: TokenType::Native,
                    id: None,
                    value,
                });
            }
        }

        // Update address stack for delegate calls
        let next_caller = match inputs.scheme {
            CallScheme::DelegateCall => from,
            _ => to,
        };
        self.address_stack.push(next_caller);

        // Create call trace entry
        let mut trace_address = Vec::new();
        if let Some(&parent_index) = self.call_stack.last() {
            trace_address = self.call_traces[parent_index].trace_address.clone();
            trace_address.push(self.call_traces[parent_index].subtraces.len());
        }

        let trace = CallTrace {
            from,
            to,
            value: inputs.call_value(),
            input: inputs.input.bytes(context),
            call_scheme: Some(inputs.scheme),
            create_scheme: None,
            gas_used: U256::ZERO,
            output: Bytes::new(),
            status: CallStatus::InProgress,
            error_origin: false,
            subtraces: Vec::new(),
            trace_address,
            slot_changes: Vec::new(), // Initialize empty slot changes
        };

        self.call_traces.push(trace);
        self.call_stack.push(self.call_traces.len() - 1);
        None
    }

    /// Processes contract creation transactions
    ///
    /// # Processing Steps
    /// 1. Records initial ETH transfer (if any)
    /// 2. Creates pending transfer record
    /// 3. Initializes creation trace entry
    /// 4. Updates call stack
    ///
    /// # Note
    /// Contract address is initially unknown and updated in create_end
    fn create(&mut self, _context: &mut CTX, inputs: &mut CreateInputs) -> Option<CreateOutcome> {
        let mut from = self.address_stack.last().copied().unwrap_or(inputs.caller);
        if from == Address::ZERO {
            // If from is zero, use the caller as the effective from address
            from = inputs.caller;
        }
        let to = Address::ZERO; // Will be updated in create_end
        self.address_stack.push(to);

        // Track initial ETH transfer
        if inputs.value > U256::ZERO {
            let transfer = TokenTransfer {
                token: NATIVE_TOKEN_ADDRESS,
                from,
                to: None, // Updated in create_end
                token_type: TokenType::Native,
                id: None,
                value: inputs.value,
            };
            self.transfers.push(transfer.clone());
            self.pending_create_transfers
                .push((self.transfers.len() - 1, transfer));
        }

        // Create trace entry
        let mut trace_address = Vec::new();
        if let Some(&parent_index) = self.call_stack.last() {
            trace_address = self.call_traces[parent_index].trace_address.clone();
            trace_address.push(self.call_traces[parent_index].subtraces.len());
        }

        let trace = CallTrace {
            from,
            to, // Updated in create_end
            value: inputs.value,
            input: inputs.init_code.clone(),
            call_scheme: None,
            create_scheme: Some(inputs.scheme),
            gas_used: U256::ZERO,
            output: Bytes::new(),
            status: CallStatus::InProgress,
            error_origin: false,
            subtraces: Vec::new(),
            trace_address,
            slot_changes: Vec::new(), // Initialize empty slot changes
        };

        self.call_traces.push(trace);
        self.call_stack.push(self.call_traces.len() - 1);
        None
    }

    /// Finalizes a contract call
    ///
    /// # Processing Steps
    /// 1. Updates call trace with results
    /// 2. Processes any error information
    /// 3. Maintains address stack
    ///
    /// # Special Handling
    /// - Delegate calls: Address stack maintained differently
    /// - Errors: Captured and formatted appropriately
    fn call_end(&mut self, _context: &mut CTX, _inputs: &CallInputs, outcome: &mut CallOutcome) {
        self.handle_end(
            outcome.result.result,
            outcome.result.gas.spent(),
            outcome.result.output.clone(),
        );
        self.address_stack.pop();
    }

    /// Finalizes contract creation
    ///
    /// # Processing Steps
    /// 1. Updates trace with actual contract address
    /// 2. Resolves pending transfer recipient
    /// 3. Updates execution status
    ///
    /// # Important
    /// This is where the contract address becomes known and
    /// all pending references are updated
    fn create_end(
        &mut self,
        _context: &mut CTX,
        _inputs: &CreateInputs,
        outcome: &mut CreateOutcome,
    ) {
        if let Some(address) = outcome.address {
            // Get current trace index without removing it
            // This will be popped in handle_end
            if let Some(trace_index) = self.call_stack.last() {
                self.call_traces[*trace_index].to = address;
            }

            // Remove and process the corresponding pending transfer
            // We pop here because this transfer is now complete
            if let Some((transfer_index, mut transfer)) = self.pending_create_transfers.pop() {
                transfer.to = Some(address);
                self.transfers[transfer_index] = transfer;
            }
        }
        // handle_end will pop the call_stack
        self.handle_end(
            outcome.result.result,
            outcome.result.gas.spent(),
            outcome.result.output.clone(),
        );
        self.address_stack.pop();
    }

    /// Processes emitted event logs
    ///
    /// # Processing Steps
    /// 1. Records all logs for complete history
    /// 2. Parses ERC20/ERC721/ERC1155 Transfer events
    /// 3. Records token transfers if detected
    ///
    /// # Note
    /// Special attention to ERC20/ERC721/ERC1155 Transfer events for
    /// accurate token transfer tracking
    fn log(&mut self, _interp: &mut Interpreter<INTR>, _context: &mut CTX, log: Log) {
        self.logs.push(log.clone());
        let mut transfers = TokenTransfer::get_token_transfers(&log);
        self.transfers.append(&mut transfers);
    }

    /// Handles contract self-destruction
    ///
    /// # Processing Steps
    /// 1. Records final balance transfer
    /// 2. Only processes non-zero value transfers
    ///
    /// # Note
    /// This is the final transfer of a contract's remaining balance
    /// before it is destroyed
    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        if value > U256::ZERO {
            self.transfers.push(TokenTransfer {
                token: NATIVE_TOKEN_ADDRESS,
                from: contract,
                to: Some(target),
                value,
                token_type: TokenType::Native,
                id: None,
            });
        }
    }
}
