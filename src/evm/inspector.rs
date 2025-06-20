//! Inspector management for TraceEvm
//!
//! This module provides inspector-specific functionality for the TraceEvm,
//! allowing access to trace data and inspector state management.

use revm::database::{CacheDB,DatabaseRef,Database};
use revm::handler::MainnetContext;
use crate::traits::TraceInspector;
use crate::evm::TraceEvm;

impl<DB, INSP> TraceEvm<CacheDB<DB>, INSP> 
where
    DB: DatabaseRef,
    INSP: TraceInspector<MainnetContext<CacheDB<DB>>>,
{
    /// Retrieve the current output from the inspector
    ///
    /// Returns the accumulated trace data or analysis results from the inspector.
    /// The exact type and content depends on the specific inspector implementation.
    ///
    /// # Returns
    /// The inspector's output data (traces, logs, analysis results, etc.)
    pub fn get_inspector_output(&self) -> INSP::Output {
        self.inspector.get_output()
    }

    /// Reset the inspector to its initial state
    ///
    /// Clears any accumulated trace data, logs, or internal state in the inspector.
    /// This should be called before processing a new transaction or batch to ensure
    /// clean state isolation.
    pub fn reset_inspector(&mut self) {
        self.inspector.reset();
    }
}

impl<DB, INSP> TraceEvm<DB, INSP> 
where
    DB: Database,
    INSP: Clone
{
    pub fn clone_inspector(&self) -> INSP {
        self.inspector.clone()
    }
}


