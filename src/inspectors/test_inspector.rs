
pub use revm::inspector::{
    NoOpInspector,
    inspectors::{GasInspector, TracerEip3155},
};
pub struct TestInspector;

use revm::{
    context::ContextTr,
    Inspector,
    interpreter::{InterpreterTypes, CallInputs, CallOutcome},
};


impl<CTX, INTR> Inspector<CTX, INTR> for TestInspector
where
    CTX: ContextTr,
    INTR: InterpreterTypes,
{
    fn call(&mut self, _context: &mut CTX, _inputs: &mut CallInputs) -> Option<CallOutcome> {
        panic!("Inspector call triggered!");
    }
}