//! Call trace handling and error tracking
//!
//! This module implements the core tracing functionality:
//! - Call hierarchy tracking
//! - Error propagation and location
//! - Execution result processing
//! - Status management
//!
//! The implementation uses a tree structure to maintain the complete
//! call hierarchy, with special handling for error cases to identify
//! the exact point of failure in complex transactions.

use crate::inspectors::tx_inspector::TxInspector;
use revm::context_interface::result::HaltReason;
use revm::interpreter::{InstructionResult, SuccessOrHalt};

use crate::types::*;
use crate::utils::error_utils::parse_custom_error;
use alloy::primitives::{hex, Bytes, U256};

impl TxInspector {
    /// Locates the trace address of the first error in the call tree
    ///
    /// Returns the position in the call tree where the first error occurred,
    /// represented as a sequence of indices into the call tree.
    ///
    /// # Returns
    /// * `Some(vec![0,1,2])` - Error occurred in the third child of the second child of the first call
    /// * `None` - No errors found
    pub fn get_error_trace_address(&self) -> Option<Vec<usize>> {
        self.find_error_trace()
            .map(|error_trace| error_trace.trace_address.clone())
    }

    /// Performs depth-first search to find the source of an error
    ///
    /// Traverses the call tree to find the deepest call that meets all criteria:
    /// - Has a failed execution status
    /// - Is marked as the origin of the error
    /// - Has no failed child calls
    ///
    /// This helps identify the exact point where an error originated,
    /// rather than where it was propagated to.
    ///
    /// # Returns
    /// * `Some(&CallTrace)` - Reference to the trace where the error originated
    /// * `None` - No errors found in the call tree
    pub fn find_error_trace(&self) -> Option<&CallTrace> {
        fn find_error_recursive(trace: &CallTrace) -> Option<&CallTrace> {
            let mut last_error = None;
            for subtrace in &trace.subtraces {
                if !subtrace.status.is_success() {
                    if let Some(error) = find_error_recursive(subtrace) {
                        last_error = Some(error);
                    }
                }
            }

            if trace.error_origin {
                Some(trace)
            } else {
                last_error
            }
        }

        let mut last_error = None;
        for trace in &self.call_traces {
            if !trace.status.is_success() {
                if let Some(error) = find_error_recursive(trace) {
                    last_error = Some(error);
                }
            }
        }
        last_error
    }

    /// Updates call trace with execution results and maintains call hierarchy
    ///
    /// This method:
    /// 1. Updates the current trace with execution results
    /// 2. Processes error status and messages
    /// 3. Identifies error origins
    /// 4. Maintains the call tree structure
    ///
    /// # Arguments
    /// * `result` - Final execution status from the EVM
    /// * `gas_used` - Total gas consumed by the call
    /// * `output` - Return data or error message
    ///
    /// # Call Tree Management
    /// - Pops the current call from the stack
    /// - Updates its execution details
    /// - Moves it to parent's subtraces if not root
    /// - Marks error origins for failed calls
    pub fn handle_end(&mut self, result: InstructionResult, gas_used: u64, output: Bytes) {
        if let Some(trace_index) = self.call_stack.pop() {
            let trace = &mut self.call_traces[trace_index];
            trace.gas_used = U256::from(gas_used);
            trace.output = output.clone();

            // Convert execution result to call status
            let status = match SuccessOrHalt::<HaltReason>::from(result) {
                SuccessOrHalt::Success(_) => CallStatus::Success,
                SuccessOrHalt::Revert => {
                    if let Some(error_msg) = parse_custom_error(&output) {
                        CallStatus::Revert(error_msg)
                    } else {
                        CallStatus::Revert(format!("0x{}", hex::encode(output)))
                    }
                }
                SuccessOrHalt::Halt(reason) => CallStatus::Halt(format!("{reason:?}")),
                SuccessOrHalt::FatalExternalError => CallStatus::FatalError,
                // Internal state is impossible here as call_end is only called after execution completion
                SuccessOrHalt::Internal(_) => CallStatus::Success,
            };

            trace.status = status;

            // Mark as error origin if this call failed but all subtraces succeeded
            trace.error_origin = !trace.status.is_success()
                && trace
                    .subtraces
                    .iter()
                    .all(|subtrace| subtrace.status.is_success());

            // Move trace to parent's subtraces if not root
            if let Some(&parent_index) = self.call_stack.last() {
                let trace = self.call_traces.remove(trace_index);
                self.call_traces[parent_index].subtraces.push(trace);
            }
        }
    }
}
