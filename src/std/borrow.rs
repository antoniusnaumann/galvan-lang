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

#[doc(hidden)]
pub fn __ref_value_eq<T: PartialEq>(left: &Arc<Mutex<T>>, right: &Arc<Mutex<T>>) -> bool {
    if Arc::ptr_eq(left, right) {
        return true;
    }

    let left_address = Arc::as_ptr(left) as usize;
    let right_address = Arc::as_ptr(right) as usize;
    if left_address < right_address {
        let left = left.lock().unwrap();
        let right = right.lock().unwrap();
        *left == *right
    } else {
        let right = right.lock().unwrap();
        let left = left.lock().unwrap();
        *left == *right
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Barrier;
    use std::thread;

    use super::*;

    #[test]
    fn ref_value_equality_compares_contents_and_handles_aliases() {
        let left = Arc::new(Mutex::new(String::from("same")));
        let equal = Arc::new(Mutex::new(String::from("same")));
        let different = Arc::new(Mutex::new(String::from("different")));

        assert!(__ref_value_eq(&left, &Arc::clone(&left)));
        assert!(__ref_value_eq(&left, &equal));
        assert!(!__ref_value_eq(&left, &different));
    }

    #[test]
    fn ref_value_equality_uses_the_same_lock_order_in_both_directions() {
        let left = Arc::new(Mutex::new(String::from("same")));
        let right = Arc::new(Mutex::new(String::from("same")));
        let barrier = Arc::new(Barrier::new(2));

        let first = {
            let left = Arc::clone(&left);
            let right = Arc::clone(&right);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                __ref_value_eq(&left, &right)
            })
        };
        let second = thread::spawn(move || {
            barrier.wait();
            __ref_value_eq(&right, &left)
        });

        assert!(first.join().unwrap());
        assert!(second.join().unwrap());
    }
}
