use std::borrow::Borrow;

#[inline(always)]
pub fn r#if<T, F>(condition: bool, body: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    if condition {
        Some(body())
    } else {
        None
    }
}

pub trait __ToOption<T: ?Sized> {
    type Inner: Borrow<T>;

    fn __to_option(&self) -> Option<Self::Inner>;

    fn __borrow_inner(&self) -> Option<&T>;

    #[inline(always)]
    fn __or_else<F>(&self, f: F) -> Self::Inner
    where
        F: FnOnce() -> Self::Inner,
    {
        self.__to_option().unwrap_or_else(f)
    }
}

impl<T: ToOwned> __ToOption<T> for Option<T> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().map(|x| x.to_owned())
    }

    fn __borrow_inner(&self) -> Option<&T> {
        self.as_ref().map(|x| x.borrow())
    }
}

impl<T: ToOwned> __ToOption<T> for &Option<T> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().map(|x| x.to_owned())
    }

    fn __borrow_inner(&self) -> Option<&T> {
        self.as_ref().map(|x| x.borrow())
    }
}

impl<T: ToOwned, _E> __ToOption<T> for Result<T, _E> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().ok().map(|x| x.to_owned())
    }

    fn __borrow_inner(&self) -> Option<&T> {
        self.as_ref().ok().map(|x| x.borrow())
    }
}

#[inline(always)]
pub fn r#try<T, O, F, B, S>(fallible: O, body: F) -> Option<T>
where
    F: FnOnce(&S) -> T,
    O: __ToOption<B>,
    B: Borrow<S>,
    S: ?Sized,
{
    fallible.__borrow_inner().map(|b| b.borrow()).map(body)
}
