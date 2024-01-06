// TODO: Improve transpilation for copy types and take bool instead of &bool here
#[inline(always)]
pub fn r#if<T, F>(condition: &bool, body: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    if *condition {
        Some(body())
    } else {
        None
    }
}

pub trait __ToOption {
    type Inner;

    fn __to_option(&self) -> Option<Self::Inner>;

    #[inline(always)]
    fn __or_else<F>(&self, f: F) -> Self::Inner
    where
        F: FnOnce() -> Self::Inner,
    {
        self.__to_option().unwrap_or_else(f)
    }
}

impl<T: ToOwned> __ToOption for Option<T> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().map(|x| x.to_owned())
    }
}

impl<T: ToOwned> __ToOption for &Option<T> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().map(|x| x.to_owned())
    }
}

impl<T: ToOwned, _E> __ToOption for Result<T, _E> {
    type Inner = T::Owned;

    #[inline(always)]
    fn __to_option(&self) -> Option<Self::Inner> {
        self.as_ref().ok().map(|x| x.to_owned())
    }
}

#[inline(always)]
pub fn r#try<T, O, F>(fallible: &O, body: F) -> Option<T>
where
    F: FnOnce(O::Inner) -> T,
    O: __ToOption + ToOwned,
{
    fallible.__to_option().map(body)
}
