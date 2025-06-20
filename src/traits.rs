use crate::types::SimulationBatch;
use crate::errors::EvmError;
use revm::context_interface::result::ExecutionResult;
use revm::inspector::Inspector;

/// Defines standard transaction processing capabilities
/// 
/// This trait establishes a standardized flow for transaction processing:
/// 1. Transaction preparation and validation
/// 2. Execution in EVM
/// 3. Result collection and state management
/// 
/// Implementors must follow this standard flow to ensure consistent behavior
/// across different execution contexts.
pub trait TransactionProcessor {
    /// Type of data collected by the inspector during execution
    type InspectorOutput;
    
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
    fn process_transactions(
        &mut self,
        batch: SimulationBatch
    ) -> Vec<Result<(ExecutionResult, Self::InspectorOutput), EvmError>>;
}



/// Defines how an inspector can reset its internal state
/// 
/// This trait is crucial for inspectors that maintain state between transactions
/// and need to clear that state before processing a new transaction.
/// 
/// # Example
/// ```
/// use revm_trace::Reset;
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
}


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
/// use revm_trace::TraceOutput;
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
/// use revm_trace_multi_thread::traits::TraceInspector;
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
/// use revm_trace_multi_thread::traits::{Reset, TraceOutput, TraceInspector};
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