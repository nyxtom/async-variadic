# async-variadic

[![Latest version](https://img.shields.io/crates/v/async-variadic.svg)](https://crates.io/crates/async-variadic)
[![Documentation](https://docs.rs/async-variadic/badge.svg)](https://docs.rs/async-variadic)
![License](https://img.shields.io/crates/l/async-variadic.svg)

Provides a way to pass along async functions with a trait that can support n-ary arguments. This code is inspired from the use of [trait specialization](https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions/) in order to support variadics. 

## Examples

```rust
use async_variadic::AsyncFn;

async fn min() {}
async fn test2(_s: String) -> i32 { 3 }
async fn test3(a: i32, b: i32) -> i32 { a + b }

fn assert_impl_fn<T>(_: impl AsyncFn<T>) {}

assert_impl_fn(min);
assert_impl_fn(test2);
assert_impl_fn(test3);
```

## actix-web handlers + use of Into/From Traits

[actix-web](https://github.com/actix/actix-web/blob/master/actix-web/src/handler.rs) has an implementation of this used for handler functions that convert [FromRequest](https://docs.rs/actix-web/latest/actix_web/trait.FromRequest.html) and return any type that implements [Responder](https://docs.rs/actix-web/latest/actix_web/trait.Responder.html). This is how you can write a web application that can bind in an arbitrary number of arguments at any position and return any type of data (so long as they implement the corresponding traits).

## Background: Conflicting Implementations

In the background to this, you may try to implement a trait variation for any **Function** that has a given number of arguments. Looking at the example provided in the [trait specialization article](https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions/)

```rust
trait VariadicFunction {
  fn call(&self, req: &[f64]) -> f64;
}

impl<Func> VariadicFunction for Func
where Func : Fn(f64)->f64 {
  fn eval(&self,args : &[f64])->f64 {
    (self)(args[0])
  }
}

impl<Func> VariadicFunction for Func
where Func : Fn(f64,f64)->f64 {
  fn eval(&self,args : &[f64])->f64 {
    (self)(args[0],args[1])
  } }

fn evaluate<F:VariadicFunction>(func : F, args: &[f64]) -> f64{
  func.eval(args)
}
```

The compiler will output something like:

```rust
error[E0119]: conflicting implementations of trait `VariadicFunction`:
  --> src/lib.rs:12:1
   |
5  | / impl<Func> VariadicFunction for Func
6  | | where Func : Fn(f64)->f64 {
7  | |   fn eval(&self,args : &[f64])->f64 {
8  | |     (self)(args[0])
9  | |   }
10 | | }
   | |_- first implementation here
11 |
12 | / impl<Func> VariadicFunction for Func
13 | | where Func : Fn(f64,f64)->f64 {
14 | |   fn eval(&self,args : &[f64])->f64 {
15 | |     (self)(args[0],args[1])
16 | |   }
17 | | }
   | |_^ conflicting implementation

error: aborting due to previous error
```

I've had similar issues when trying to create a **Middleware** trait that can handle a varying number of arguments. I wanted to do this specifically because I thought it might be nice to pass in closure that only specified one or more of the arguments.

```rust
#[async_trait]
pub trait Middleware<'a, 'b>: Send + Sync {
    #[must_use = "handle future must be used"]
    async fn handle(&self, request: &'a mut Request, response: &'b mut Response);
}

#[async_trait]
impl<'a, 'b, F, Fut, Res> Middleware<'a, 'b> for F
where
    F: Send + Sync + 'a + Fn(&'a mut Request, &'b mut Response) -> Fut,
    Fut: Future<Output = Res> + Send + 'b,
{
    async fn handle(&self, request: &'a mut Request, response: &'b mut Response) {
        (self)(request, response).await;
    }
}
```

Here I am bounding the `F` type to a `Fn` trait that takes two arguments. If I wanted to specify a different closure that only passed along the request that would look like this:

```rust
#[async_trait]
pub trait Middleware<'a, 'b>: Send + Sync {
    #[must_use = "handle future must be used"]
    async fn handle(&self, request: &'a mut Request, response: &'b mut Response);
}

#[async_trait]
impl<'a, 'b, F, Fut, Res> Middleware<'a, 'b> for F
where
    F: Send + Sync + 'a + Fn(&'a mut Request, &'b mut Response) -> Fut,
    Fut: Future<Output = Res> + Send + 'b,
{
    async fn handle(&self, request: &'a mut Request, response: &'b mut Response) {
        (self)(request, response).await;
    }
}

#[async_trait]
impl<'a, 'b, F, Fut, Res> Middleware<'a, 'b> for F
where
    F: Send + Sync + 'a + Fn(&'a mut Request) -> Fut,
    Fut: Future<Output = Res> + Send + 'b,
{
    async fn handle(&self, request: &'a mut Request, response: &'b mut Response) {
        (self)(request).await;
    }
}
```

Unfortunately I get the same compilation error as above. The main reason (as discussed in the article and in [issue #60074](https://github.com/rust-lang/rust/issues/60074#issuecomment-484478859)) is that a closure **could** implement both *Fn* traits and create undefined behavior.

```rust
impl FnOnce<(u32,)> for Foo {
    type Output = u32;
    extern "rust-call" fn call_once(self, args: (u32,)) -> Self::Output {
        args.0
    }
}

impl FnOnce<(u32, u32)> for Foo {
    type Output = u32;
    extern "rust-call" fn call_once(self, args: (u32, u32)) -> Self::Output {
        args.0
    }
}
```

So that doesn't work for us. We need to find another way!

### Trait Specialization

As mentioned in the article, the way around this is to create a trait specialization type that takes **Args**.

```rust
trait VariadicFunction<ArgList> {
  fn eval(&self, args: &[f64]) -> f64;
}
```

Then we can implement a **Fn** trait bound for each of the positional arguments while providing that type to the VariadicFunction.

```rust
impl<Func> VariadicFunction<f64> for Func
where Func : Fn(f64)->f64 {
  fn eval(&self,args : &[f64])->f64 {
    (self)(args[0])
  }
}

impl<Func> VariadicFunction<(f64,f64)> for Func
where Func : Fn(f64,f64)->f64 {
  fn eval(&self,args : &[f64])->f64 {
    (self)(args[0],args[1])
  }
}

fn evaluate<ArgList, F>(func : F, args: &[f64]) -> f64
where F: VariadicFunction<ArgList>{
  func.eval(args)
}
```

Notice that the VariadicFunction trait is actually 2 different traits once it becomes monomorphized by the compiler. We're implementing the entirely different traits for the **functions** that have the different arguments. Let's expand this for async!

## Abstracted out to AsyncFn<T>

An async function is nothing more than a function that returns a  [Future](https://doc.rust-lang.org/stable/std/future/trait.Future.html).

```rust
pub trait Future {
    type Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

The associated output is the result of the `Poll<Self::Output>` returning `Poll::Ready(output)`. We can define an `AsyncFn` trait that looks similar to the **Future** trait except it's main purpose is to be a trait bound for functions.

```rust
pub trait AsyncFn<Args> {
    type Output;
    type Future: Future<Output = Self::Output>;

    fn call(&self, args: Args) -> Self::Future;
}
```

Now all that's left is implementing the trait for different closures. The simplest case is going to be an empty async function.

```rust
impl<Func, Fut> AsyncFn<()> for Func
where
    Func: Fn() -> Fut,
    Fut: Future
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, args: ()) -> Self::Future {
        (self)()
    }
}
```

The second case is going to be for a function that takes 1 argument. **Notice** that we don't have to specify the output since the **Future::Output** already specifies that for us.

```rust
impl<Func, Fut, A> AsyncFn<(A,)> for Func
where
    Func: Fn(A) -> Fut,
    Fut: Future
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, args: (A,)) -> Self::Future {
        let (a,) = args;
        (self)(a)
    }
}
```

We can amend the **call** function here to destructure the arguments tuple into the local variable.

```rust
impl<Func, Fut, A> AsyncFn<(A,)> for Func
where
    Func: Fn(A) -> Fut,
    Fut: Future
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, (A,): (A,)) -> Self::Future {
        (self)(A)
    }
}
```

This can be expanded upon to support more than one argument similar to the above implementation.

```rust
impl<Func, Fut, A, B> AsyncFn<(A,B)> for Func
where
    Func: Fn(A) -> Fut,
    Fut: Future
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call(&self, (A,B): (A,B)) -> Self::Future {
        (self)(A, B)
    }
}
```

### Automatically Implementing with a Macro

While it's simple enough to implement each of these variations manually, we can also write a simple macro to automatically build these for us.

```rust
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
```

This macro simply takes the `$(param:ident)*` pattern and specifies it as both the generic argument to the `impl<>`, the trait specialization in the `AsyncFn<T>`, the trait bound on the `Func` arguments, and finally in the actual **call** function. Now all we need to do is call this macro with different identifiers and it's good to go.

```rust
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
```

This will let us specify up to 12 different parameters. Each new generic identifier will be included in an entirely new trait implementation.

## Tests

The tests below show us how we can now specify any kind of async function that captures any type of variable (in any order) and returns any type (as indicated by the **Future::Output**). 

```rust
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
```

The main use case here is now we can have a function like `assert_impl_fn<T>` that actually takes an async function or closure as an argument! This is especially useful for our handlers, middleware and web server implementations.
