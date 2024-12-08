//! Core traits for revm-trace functionality
//! 
//! This module defines the fundamental traits that power the tracing system:
//! - Reset: For managing inspector state
//! - TraceOutput: For converting inspector state to output
//! - TraceInspector: Combined trait for full inspector functionality

/// Re-export core REVM traits for user convenience
/// 
/// These re-exports allow users to access essential REVM traits directly
/// through revm-trace, without having to:
/// - Add explicit revm dependency in their Cargo.toml
/// - Manage version compatibility between revm and revm-trace
/// - Import traits from multiple crates
/// 
/// # Example
/// ```
/// use revm_trace::{Inspector, Database};
/// 
/// struct MyInspector;
/// 
/// impl<DB: Database> Inspector<DB> for MyInspector {
///     // Implementation...
/// }
/// ```
/// 
/// Instead of:
/// ```ignore
/// // In Cargo.toml:
/// // revm = "x.y.z"  // Need to ensure version compatibility
/// 
/// use revm::{Inspector, Database};
/// ```
pub use revm::{Inspector, Database,GetInspector};

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

/// Combined trait for full inspector functionality
/// 
/// This trait combines the core REVM `Inspector` trait with our custom
/// `Reset` and `TraceOutput` traits to provide complete tracing capabilities.
/// 
/// # Type Parameters
/// * `DB` - The database type used by the inspector
/// 
/// # Requirements
/// Implementing types must satisfy:
/// - REVM's `Inspector<DB>` trait for basic inspection
/// - `Reset` for state management
/// - `TraceOutput` for result formatting
pub trait TraceInspector<DB>: Inspector<DB> + Reset + TraceOutput 
where 
    DB: Database
{}

/// Blanket implementation for any type implementing required traits
/// 
/// This implementation automatically provides `TraceInspector` for any type
/// that implements all the required traits, reducing boilerplate code.
impl<T, DB> TraceInspector<DB> for T 
where 
    DB: Database,
    T: Inspector<DB> + Reset + TraceOutput
{}