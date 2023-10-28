use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;

trait Type: Clone {}

type LocalVal<'a, T> = Cow<'a, T>;
type StoredVal<T> = T;
type LocalRef<'a, T> = &'a T;
type StoredRef<T> = Arc<Mutex<T>>;

trait CreateStoredRef {
    type Stored: Type;
    fn create(stored: Self::Stored) -> Self;
}

impl<T: Type> CreateStoredRef for StoredRef<T> {
    type Stored = T;

    fn create(stored: Self::Stored) -> Self {
        Arc::new(Mutex::new(stored))
    }
}

trait AsLocalVal {
    type Return: AsStoredVal + AsLocalVal + ToOwned;
    fn as_local_val(&self) -> LocalVal<Self::Return>;
}

impl<T: Type> AsLocalVal for LocalVal<'_, T> {
    type Return = T;
    fn as_local_val(&self) -> LocalVal<Self::Return> {
        self.clone()
    }
}

impl<T: Type> AsLocalVal for T {
    type Return = T;
    fn as_local_val(&self) -> LocalVal<Self::Return> {
        Cow::Borrowed(self)
    }
}

trait AsStoredVal {
    type Stored: Type;
    fn as_stored_val(&self) -> StoredVal<Self::Stored>;
}

impl<T: Type> AsStoredVal for LocalVal<'_, T> {
    type Stored = T;

    fn as_stored_val(&self) -> StoredVal<Self::Stored> {
        self.as_ref().clone()
    }
}

impl<T: Type> AsStoredVal for T {
    type Stored = T;

    fn as_stored_val(&self) -> StoredVal<Self::Stored> {
        self.clone()
    }
}

trait AsLocalRef {
    type Return: Type;
    fn as_local_ref(&self) -> LocalRef<Self::Return>;
}
// TODO: Blanket Implementations

trait AsStoredRef {
    type Stored: Type;
    fn as_stored_ref(&self) -> StoredRef<Self::Stored>;
}

/// Enables implicitly cloning stored refs
impl<T: Type> AsStoredRef for StoredRef<T> {
    type Stored = T;

    fn as_stored_ref(&self) -> StoredRef<Self::Stored> {
        StoredRef::clone(self)
    }
}

/// This conversion must be invoked explicitly by using the copy keyword or the copy operator <:
impl<T: Type> AsStoredRef for StoredVal<T> {
    type Stored = T;

    fn as_stored_ref(&self) -> StoredRef<Self::Stored> {
        StoredRef::create(self.clone())
    }
}

/// This conversion must be invoked explicitly by using the copy keyword or the copy operator <:
impl<T: Type> AsStoredRef for LocalVal<'_, T> {
    type Stored = T;

    fn as_stored_ref(&self) -> StoredRef<Self::Stored> {
        StoredRef::create(self.as_ref().clone())
    }
}

#[cfg(test)]
mod test {
    use super::{AsLocalRef, AsLocalVal, AsStoredRef, AsStoredVal, Type};
    use crate::StoredRef;

    #[derive(Clone)]
    struct TypeA {}
    // TODO: derive macro
    impl Type for TypeA {}

    #[derive(Clone)]
    struct TypeB {}
    // TODO: derive macro
    impl Type for TypeB {}

    struct MyType {
        a: TypeA,
        b: StoredRef<TypeB>,
    }

    fn make_t<A>(a: A, b: StoredRef<TypeB>) -> MyType
    where
        A: AsStoredVal<Stored = TypeA> + AsLocalVal,
    {
        MyType {
            a: a.as_stored_val(),
            b: b.as_stored_ref(),
        }
    }

    fn print<T, A>(a: A)
    where
        A: AsStoredVal<Stored = T> + AsLocalVal,
        T: Type,
    {
        let a = a.as_local_val();
        let b = a.as_local_val();
    }

    #[test]
    fn main() {
        let a = TypeA {};
        let b = TypeB {};

        let a = a.as_local_val();
        let b = b.as_local_val();
        let t = MyType {
            a: a.as_stored_val(),
            b: b.as_stored_ref(),
        };

        print(t.a.as_local_val());

        let c = &a;

        let t_new = make_t(t.a.as_local_val(), t.b.as_stored_ref());

        let x = a.as_local_val();
        let y = c.as_local_val();
        print(x.as_local_val());
        // To disambiguate type checking for this case, we should add turbofish to all generated Cows by tracing the types somehow
        let z = y.as_local_val();

        print(z.as_local_val());
    }
}
