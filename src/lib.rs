use std::future::Future;

pub trait AsyncFn<Args> {
    type Output;
    type Future: Future<Output = Self::Output>;

    fn call(&self, args: Args) -> Self::Future;
}

/// Generates a [`AsyncFn`] trait impl for N-ary functions where N is specified with a
/// space separated type parameters.
///
/// # Examples
/// ```ignore
/// ary! {}        // implements Handler for types: fn() -> R
/// ary! { A B C } // implements Handler for types: fn(A, B, C) -> R
/// ```
macro_rules! ary ({ $($param:ident)* } => {
    impl<Func, Fut, $($param,)*> AsyncFn<($($param,)*)> for Func
    where
        Func: Fn($($param),*) -> Fut,
        Fut: Future
    {
        type Output = Fut::Output;
        type Future = Fut;

        #[inline]
        #[allow(non_snake_case)]
        fn call(&self, ($($param,)*): ($($param,)*)) -> Self::Future {
            (self)($($param,)*)
        }
    }
});

ary! {}
ary! { A }
ary! { A B }
ary! { A B C }
ary! { A B C D }
ary! { A B C D E }
ary! { A B C D E F }
ary! { A B C D E F G }
ary! { A B C D E F G H }
ary! { A B C D E F G H I }
ary! { A B C D E F G H I J }
ary! { A B C D E F G H I J K }
ary! { A B C D E F G H I J K L }

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_impl_fn<T>(_: impl AsyncFn<T>) {}

    #[test]
    fn test_args() {
        async fn min() {}
        async fn min_output() -> i32 {
            4
        }
        async fn with_req(_req: String) -> &'static str {
            "foo"
        }
        #[rustfmt::skip]
        #[allow(clippy::too_many_arguments, clippy::just_underscores_and_digits)]
        async fn max(
            _01: (), _02: (), _03: (), _04: (), _05: (), _06: (),
            _07: (), _08: (), _09: (), _10: (), _11: (), _12: (),
        ) {}

        assert_impl_fn(min);
        assert_impl_fn(with_req);
        assert_impl_fn(min_output);
        assert_impl_fn(max);
    }
}
