use crate::errors::EvmError;
use crate::types::{SimulationBatch, SlotChange};
use revm::context_interface::result::ExecutionResult;
use revm::inspector::{Inspector, NoOpInspector};
use std::collections::HashMap;
use alloy::primitives::Address;

/// Defines how an inspector converts its state to a specific output type
///
/// This trait allows inspectors to provide their collected data in a
/// standardized format, making it easier to process and analyze results.
///
/// # Type Parameters
/// * `Output` - The type that this inspector produces as its final result
///
/// # Example
/// ```
/// use revm_trace::traits::TraceOutput;
///
/// struct MyInspector {
///     gas_used: u64,
/// }
///
/// impl TraceOutput for MyInspector {
///     type Output = u64;
///     
///     fn get_output(&self) -> Self::Output {
///         self.gas_used
///     }
/// }
/// ```
pub trait TraceOutput {
    /// The type of output this inspector produces
    type Output;

    /// Converts the current inspector state into the output type
    ///
    /// This method should collect all relevant information from the
    /// inspector and return it in the specified output format.
    fn get_output(&self) -> Self::Output;
}

/// Defines how an inspector can reset its internal state
///
/// This trait is crucial for inspectors that maintain state between transactions
/// and need to clear that state before processing a new transaction.
///
/// # Example
/// ```
/// use revm_trace::traits::Reset;
///
/// struct MyInspector {
///     call_count: u32,
///     gas_used: u64,
/// }
///
/// impl Reset for MyInspector {
///     fn reset(&mut self) {
///         self.call_count = 0;
///         self.gas_used = 0;
///     }
/// }
/// ```
pub trait Reset {
    /// Resets the inspector to its initial state
    ///
    /// This method should clear any accumulated state or metrics,
    /// preparing the inspector for a new transaction.
    fn reset(&mut self);


    /// Resets the slot cache used by the inspector
    fn reset_slot_cache(&mut self);
}

/// Combined trait for EVM inspectors with tracing capabilities
///
/// `TraceInspector` is a marker trait that combines three essential traits
/// for comprehensive EVM transaction tracing:
///
/// - **`Inspector<CTX>`**: Core REVM inspector interface for receiving EVM execution callbacks
/// - **`Reset`**: Ability to reset internal state between transactions
/// - **`TraceOutput`**: Ability to extract structured output after execution
///
/// Any type implementing all three traits automatically implements `TraceInspector`.
///
/// # Usage
///
/// This trait is primarily used as a constraint in generic functions and structs
/// that need full tracing capabilities:
///
/// ```rust
/// use revm_trace::traits::TraceInspector;
/// use revm::database::Database;
///
/// fn process_with_trace<DB, I>(inspector: &mut I)
/// where
///     DB: Database,
///     I: TraceInspector<DB>
/// {
///     // Can use inspector for EVM execution
///     // Can reset state between transactions  
///     // Can extract output after execution
/// }
/// ```
///
/// # Examples
///
/// Implementing `TraceInspector` for a custom inspector:
///
/// ```rust
/// use revm_trace::traits::{Reset, TraceOutput, TraceInspector};
/// use revm::{Inspector, database::Database};
///
/// struct MyInspector {
///     call_count: u32,
/// }
///
/// impl<DB: Database> Inspector<DB> for MyInspector {
///     // Inspector implementation...
/// }
///
/// impl Reset for MyInspector {
///     fn reset(&mut self) {
///         self.call_count = 0;
///     }
/// }
///
/// impl TraceOutput for MyInspector {
///     type Output = u32;
///     
///     fn get_output(&self) -> Self::Output {
///         self.call_count
///     }
/// }
///
/// // MyInspector now automatically implements TraceInspector
/// ```
pub trait TraceInspector<CTX>: Inspector<CTX> + Reset + TraceOutput {}

/// Automatic implementation of `TraceInspector` for qualifying types
///
/// This blanket implementation automatically provides `TraceInspector` for any type
/// that implements all the required traits. This design follows Rust's principle
/// of "coherence" and eliminates the need for manual implementation boilerplate.
///
/// # Required Traits
///
/// Any type implementing all of the following traits will automatically
/// implement `TraceInspector`:
///
/// - **`Inspector<CTX>`**: Core REVM inspector interface for EVM execution callbacks
/// - **`Reset`**: Ability to reset internal state between transactions
/// - **`TraceOutput`**: Ability to extract structured output after execution  
///
/// Note: The `Clone` requirement was removed in v3.1 for better performance.
///
/// # Design Rationale
///
/// This approach provides several benefits:
///
/// 1. **Zero Boilerplate**: No need to explicitly implement `TraceInspector`
/// 2. **Automatic Coherence**: Any type meeting the requirements gets the trait automatically
/// 3. **Compile-Time Safety**: The type system ensures all required capabilities are present
/// 4. **Future Compatibility**: New requirements can be added to the supertrait bounds
///
/// # Example
///
/// ```rust
/// use revm_trace::traits::{Reset, TraceOutput, TraceInspector};
/// use revm::Inspector;
///
/// #[derive(Default)]
/// struct MyInspector {
///     calls: Vec<String>,
/// }
///
/// impl<CTX> Inspector<CTX> for MyInspector {
///     // Inspector implementation...
/// }
///
/// impl Reset for MyInspector {
///     fn reset(&mut self) {
///         self.calls.clear();
///     }
/// }
///
/// impl TraceOutput for MyInspector {
///     type Output = Vec<String>;
///     
///     fn get_output(&self) -> Self::Output {
///         self.calls.clone()
///     }
/// }
///
/// // MyInspector now automatically implements TraceInspector<CTX>
/// // No explicit implementation needed!
/// ```
///
/// # Trait Bounds
///
/// Previously required `Clone` bound for batch transaction processing,
/// but this has been optimized away in v3.1 for better performance.
impl<CTX, T> TraceInspector<CTX> for T where T: Inspector<CTX> + Reset + TraceOutput {}

impl Reset for () {
    fn reset(&mut self) {
        // No-op for unit type
    }

    fn reset_slot_cache(&mut self) {
         // No-op for unit type
    }
}

impl TraceOutput for () {
    type Output = ();

    fn get_output(&self) -> Self::Output {
        // No output for unit type
    }
}
pub type StorageDiff = HashMap<Address, Vec<SlotChange>>;
pub type TraceResult<T> = Result<(ExecutionResult, StorageDiff, T), EvmError>;

/// Defines standard transaction processing capabilities
///
/// This trait establishes a standardized flow for transaction processing:
/// 1. Transaction preparation and validation
/// 2. Execution in EVM
/// 3. Result collection and state management
///
/// Implementors must follow this standard flow to ensure consistent behavior
/// across different execution contexts.
pub trait TransactionTrace<CTX> {
    /// Type of data collected by the inspector during execution
    type Inspector: TraceInspector<CTX>;

    /// Process a batch of transactions following the standard flow
    ///
    /// # Standard Flow
    /// 1. **Preparation**
    ///    - Configure block environment
    ///    - Reset necessary states
    ///
    /// 2. **Execution**
    ///    - Process all transactions
    ///    - Collect execution results
    ///    - Gather inspector data
    ///
    /// 3. **State Management**
    ///    - Handle state persistence
    ///    - Reset states as needed
    ///
    /// # Arguments
    /// * `batch` - Contains transactions and execution parameters
    ///
    /// # Returns
    /// * Vector of results for each transaction
    fn trace_transactions(
        &mut self,
        batch: SimulationBatch,
    ) -> Vec<TraceResult<<Self::Inspector as TraceOutput>::Output>>;
}

/// Defines the ability to reset database state
///
/// This trait is implemented by EVM instances that support resetting
/// their underlying database to a clean state, typically used for:
///
/// - Batch processing where each transaction should start from the same state
/// - Testing scenarios requiring clean state isolation
/// - Simulation environments needing state rollback capabilities
pub trait ResetDB {
    /// Resets the database to its initial state
    ///
    /// This operation should:
    /// - Clear any cached modifications
    /// - Restore the database to its baseline state
    /// - Preserve the original data source connection
    fn reset_db(&mut self);
}

// ========================= NoOpInspector Implementations =========================

/// Implementation of Reset trait for NoOpInspector
///
/// Since NoOpInspector doesn't maintain any internal state that needs to be reset,
/// this is a no-operation implementation. This allows NoOpInspector to satisfy
/// the Reset trait requirement for TraceInspector.
impl Reset for NoOpInspector {
    fn reset(&mut self) {
        // No operation for NoOpInspector - it has no state to reset
    }

    /// Resets the slot cache used by NoOpInspector
    fn reset_slot_cache(&mut self) {
        // No operation for NoOpInspector - it has no slot cache to reset
    }
}

/// Implementation of TraceOutput trait for NoOpInspector
///
/// NoOpInspector produces no meaningful output, so this implementation returns
/// the unit type `()`. This allows NoOpInspector to satisfy the TraceOutput
/// trait requirement for TraceInspector.
impl TraceOutput for NoOpInspector {
    type Output = ();

    fn get_output(&self) -> Self::Output {
        // No output for NoOpInspector - returns unit type
    }
}

pub trait ResetBlock {
    /// The error type returned when block reset operations fail
    type Error;

    /// Resets the block environment to its initial state
    ///
    /// This operation should clear any modifications made to the block context,
    /// restoring it to a clean state for new transactions.
    ///
    /// # Arguments
    /// * `block_number` - The block number to reset to
    ///
    /// # Errors
    ///
    /// Returns an error if the block reset operation fails.
    fn reset_block(&mut self, block_number: u64) -> Result<(), Self::Error>;
}

// Note: NoOpInspector automatically implements TraceInspector<CTX> through the blanket implementation:
// impl<CTX, T> TraceInspector<CTX> for T
// where T: Inspector<CTX> + Reset + TraceOutput
//
// Since NoOpInspector already implements Inspector<CTX> (from revm),
// and we've implemented Reset and TraceOutput above, it automatically gets TraceInspector.
