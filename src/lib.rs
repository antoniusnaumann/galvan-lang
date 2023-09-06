use parking_lot::Mutex;
use std::{
    borrow::{Borrow, BorrowMut, Cow},
    sync::Arc,
};

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

/// A variable that follows value semantics and Clone-on-Write behavior
struct Val<'a, T: Clone>(Cow<'a, T>);
impl<'a, T: Clone> Borrow<T> for Val<'a, T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<'a, T: Clone> From<&'a StoredVal<T>> for Val<'a, T> {
    fn from(val: &'a StoredVal<T>) -> Self {
        Val(Cow::Borrowed(val.borrow()))
    }
}

impl<T: Clone> ToOwned for Val<'_, T> {
    type Owned = StoredVal<T>;

    fn to_owned(&self) -> Self::Owned {
        todo!()
    }
}

/// The owned version of a variable
struct StoredVal<T: Clone>(T);
impl<T: Clone> Borrow<T> for StoredVal<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> BorrowMut<T> for StoredVal<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A variable that follows reference semantics and is mutable
/// As opposed to Rust, references are not neccessarily exclusive
trait Ref {}

///
trait StoredRef {}

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

    use std::{
        borrow::Cow,
        sync::{Arc, Mutex},
    };
    #[derive(Clone)]
    struct TypeA {}
    #[derive(Clone)]
    struct TypeB {}

    struct MyType {
        a: TypeA,
        b: Arc<Mutex<TypeB>>,
    }

    fn make_t<A>(a: &A, b: Arc<Mutex<TypeB>>) -> MyType
    where
        A: ToOwned<Owned = TypeA>,
    {
        MyType {
            a: a.to_owned(),
            b: b.clone(),
        }
    }

    fn print<T, A>(a: A)
    where
        T: 'static,
        A: ToOwned<Owned = T>,
    {
        let a: Cow<A> = Cow::Borrowed(&a);
    }

    #[test]
    fn main() {
        let a = TypeA {};
        let b = TypeB {};
        let t = MyType {
            a: a.clone(),
            b: Arc::new(Mutex::new(b.clone())),
        };

        print(&t.a);

        let t_new = make_t(&t.a, t.b.clone());

        let x = Cow::Borrowed(&a);
        print(&x);
    }
}
