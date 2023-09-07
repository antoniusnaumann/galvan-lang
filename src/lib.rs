use std::{
    borrow::{Borrow, Cow},
    sync::Arc,
};

use parking_lot::Mutex;

trait Type {}

type LocalVal<'a, T> = Cow<'a, T>;
type StoredVal<T> = T;
type LocalRef<'a, T> = &'a T;
type StoredRef<T> = Arc<Mutex<T>>;

trait AsLocalVal {
    type Return: AsStoredVal + AsLocalVal + ToOwned;
    fn as_local_val(&self) -> LocalVal<Self::Return>;
}

impl<T: Type + Clone> AsLocalVal for LocalVal<'_, T> {
    type Return = T;
    fn as_local_val(&self) -> LocalVal<Self::Return> {
        self.clone()
    }
}

impl<T: Type + Clone> AsLocalVal for T {
    type Return = T;
    fn as_local_val(&self) -> LocalVal<Self::Return> {
        Cow::Borrowed(self.borrow())
    }
}

trait AsStoredVal {
    type Stored: Type;
    fn as_stored_val(&self) -> StoredVal<Self::Stored>;
}

impl<T: Type + Clone> AsStoredVal for LocalVal<'_, T> {
    type Stored = T;

    fn as_stored_val(&self) -> StoredVal<Self::Stored> {
        self.as_ref().clone()
    }
}

impl<T: Type + Clone> AsStoredVal for T {
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
// TODO: Blanket Implementations

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::{AsLocalRef, AsLocalVal, AsStoredRef, AsStoredVal, Type};

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
        b: Arc<Mutex<TypeB>>,
    }

    fn make_t<A>(a: &A, b: Arc<Mutex<TypeB>>) -> MyType
    where
        A: AsStoredVal<Stored = TypeA> + AsLocalVal,
    {
        MyType {
            a: a.as_stored_val(),
            b: b.clone(),
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
        let t = MyType {
            a: a.as_stored_val(),
            b: b.as_stored_ref(),
        };

        print(t.a.as_local_val());

        let c = &a;

        let t_new = make_t(&t.a, t.b.clone());

        let x = a.as_local_val();
        let y = c.as_local_val();
        print(x.as_local_val());
        // To disambiguate type checking for this case, we should add turbofish to all generated Cows by tracing the types somehow
        let z = y.as_local_val();

        print(z.as_local_val());
    }
}
