use std::fmt::{Debug, Formatter, Result};

pub struct DebugFn<F>(pub F);
impl<F: Fn(&mut Formatter<'_>) -> Result> Debug for DebugFn<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0(f)
    }
}

pub fn debug_fn<F: Fn(&mut Formatter<'_>) -> Result>(f: F) -> DebugFn<F> {
    DebugFn(f)
}
