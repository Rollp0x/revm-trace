//! Inspector management functionality for TraceEvm
//! 
//! Provides methods for:
//! - Inspector state management
//! - Output collection
//! - State reset operations

use crate::traits::{GetInspector, Reset, TraceOutput};
use crate::types::InspectorDB;
use alloy::{
    providers::Provider,
    transports::Transport,
};

use super::TraceEvm;

/// Inspector management implementation
impl<'a, T, P, I> TraceEvm<'a, T, P, I>
where
    T: Transport + Clone,
    P: Provider<T>,
    I: 'a + GetInspector<InspectorDB<T, P>>,
{
    /// Resets the inspector's internal state
    /// 
    /// This method should be called:
    /// - Before processing a new transaction
    /// - When switching between independent transactions in a batch
    /// - Any time the inspector state needs to be cleared
    /// 
    /// # Type Parameters
    /// * `I: Reset` - Inspector must implement the Reset trait
    /// 
    /// # Returns
    /// * `&mut Self` - Returns self for method chaining
    pub fn reset_inspector(&mut self) -> &mut Self
    where
        I: Reset,
    {
        self.0.context.external.reset();
        self
    }

    /// Retrieves the inspector's collected output
    /// 
    /// This method should be called:
    /// - After transaction execution completes
    /// - When analysis results are needed
    /// - Before inspector state is reset
    /// 
    /// # Type Parameters
    /// * `I: TraceOutput` - Inspector must implement the TraceOutput trait
    /// 
    /// # Returns
    /// * `I::Output` - The inspector's collected execution data
    pub fn get_inspector_output(&mut self) -> I::Output
    where
        I: TraceOutput,
    {
        self.0.context.external.get_output()
    }
}