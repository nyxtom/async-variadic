use async_trait::async_trait;
use std::future::Future;

#[async_trait]
pub trait AsyncFn<Args, Output> {
    async fn call(&self, args: Args) -> Output;
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
    #[async_trait::async_trait]
    impl<Func, Fut, $($param:Send + 'static,)*> AsyncFn<($($param,)*), Fut::Output> for Func
    where
        Func: Send + Sync + Fn($($param),*) -> Fut,
        Fut: Future + Send
    {
        #[inline]
        #[allow(non_snake_case)]
        async fn call(&self, ($($param,)*): ($($param,)*)) -> Fut::Output {
            (self)($($param,)*).await
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

    struct Request {}
    struct Response {}

    struct Body {}

    impl From<Request> for (Request,) {
        fn from(req: Request) -> Self {
            (req,)
        }
    }

    impl From<Request> for (Body,) {
        fn from(req: Request) -> Self {
            (Body {},)
        }
    }

    impl From<&'static str> for Response {
        fn from(s: &'static str) -> Self {
            Response {}
        }
    }

    fn assert_impl_fn<T, O>(_: impl AsyncFn<T, O>) {}

    fn assert_impl_output<T, O: Into<Response>>(_: impl AsyncFn<T, O>)
    where
        T: From<Request>,
    {
    }

    #[test]
    fn test_args() {
        async fn min() {}
        async fn min_output() -> i32 {
            4
        }
        async fn with_req(_req: String) -> &'static str {
            "foo"
        }
        async fn with_refs(_r: &str, _b: &[u8]) -> &'static str {
            "asdf"
        }
        struct Test {
            a: bool,
            b: u8,
        }

        impl Test {
            async fn bleh(&self) -> &u8 {
                &self.b
            }
        }

        let t = Test { a: true, b: 8 };

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
        assert_impl_fn(with_refs);
        assert_impl_fn(Test::bleh);

        async fn with_request_resp(req: Request) -> &'static str {
            "hello"
        }

        async fn with_body_resp(body: Body) -> &'static str {
            "hello"
        }

        assert_impl_output(with_request_resp);
        assert_impl_output(with_body_resp);
    }
}
