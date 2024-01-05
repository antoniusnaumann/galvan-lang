use std::borrow::{Borrow, BorrowMut};
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};

pub trait __ToRef {
    type Inner;
    fn __to_ref(&self) -> Arc<Mutex<Self::Inner>>;
}

impl<T: ToOwned> __ToRef for &T {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_ref(&self) -> Arc<Mutex<Self::Inner>> {
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

pub trait __Borrow<'a> {
    type Inner;
    fn __borrow(&'a self) -> Self::Inner;
}

impl<'a, T: Borrow<T>> __Borrow<'a> for &'a T {
    type Inner = &'a T;
    #[inline(always)]
    fn __borrow(&'a self) -> Self::Inner {
        (**self).borrow()
    }
}

impl<'a, T: 'a> __Borrow<'a> for Arc<Mutex<T>> {
    type Inner = MutexGuard<'a, T>;
    #[inline(always)]
    fn __borrow(&'a self) -> Self::Inner {
        self.lock().unwrap()
    }
}

impl<'a, T: 'a> __Borrow<'a> for MutexGuard<'a, T> {
    type Inner = &'a T;
    #[inline(always)]
    fn __borrow(&'a self) -> Self::Inner {
        (*self).deref()
    }
}

pub trait __Mut<'a> {
    type Inner;

    fn __mut(&'a mut self) -> Self::Inner;
}

impl<'a, T: BorrowMut<T> + 'a> __Mut<'a> for T {
    type Inner = &'a mut T;

    fn __mut(&'a mut self) -> Self::Inner {
        self.borrow_mut()
    }
}
