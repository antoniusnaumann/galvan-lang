use parking_lot::Mutex;
use std::sync::Arc;

trait Wrapped<T>: Clone {}

impl<T: Copy> Wrapped<T> for T {}

impl<T> Wrapped<T> for Arc<Mutex<T>> {}

trait WrapCopy {
    fn wrap_copy(self) -> Self;
}

trait Wrap<T: Clone> {
    fn wrap(self) -> T;
}

impl<T: Copy> WrapCopy for T {
    fn wrap_copy(self) -> T {
        self
    }
}

impl<T: Copy> Wrap<T> for T {
    fn wrap(self) -> T {
        self
    }
}

impl<T> Wrap<Arc<Mutex<T>>> for T {
    fn wrap(self) -> Arc<Mutex<T>> {
        Arc::new(Mutex::new(self))
    }
}

#[cfg(test)]
mod test {
    use crate::{Wrap, WrapCopy, Wrapped};

    struct ComplexType {
        _inner: i64,
    }

    fn example_method(
        _integer: impl Wrapped<i32>,
        _string: impl Wrapped<String>,
        _complex: impl Wrapped<ComplexType>,
    ) {
    }

    #[test]
    fn test_specialization() {
        let integer = 52;
        let string = String::from("Wow");
        let complex = ComplexType { _inner: 7 };

        let input_int = integer.wrap_copy();
        let input_string = string.wrap();
        let input_complex = complex.wrap();

        let arc_ref = &input_complex;
        let _arc_copied = arc_ref.clone();

        example_method(input_int, input_string, input_complex);
    }
}
