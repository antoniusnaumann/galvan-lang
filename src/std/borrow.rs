use std::ops::Deref;
use std::sync::{Arc, Mutex};

pub trait __ToRef {
    type Inner;
    fn __to_ref(&self) -> Arc<Mutex<Self::Inner>>;
}

impl<T: ToOwned> __ToRef for &T {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_ref(&self) -> Arc<Mutex<Self::Inner>> {
        #[allow(suspicious_double_ref_op)]
        Arc::new(Mutex::new(self.deref().to_owned()))
    }
}

impl<T> __ToRef for Arc<Mutex<T>> {
    type Inner = T;

    #[inline(always)]
    fn __to_ref(&self) -> Arc<Mutex<Self::Inner>> {
        Arc::clone(self)
    }
}
