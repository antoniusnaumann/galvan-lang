mod borrow;
pub use borrow::*;

mod control_flow;
pub use control_flow::*;

mod result;
pub use result::*;

// External re-exports
pub use itertools::*;
pub trait ItertoolsExt: Itertools {
    fn vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
}

impl<T: ?Sized> ItertoolsExt for T where T: Itertools {}
