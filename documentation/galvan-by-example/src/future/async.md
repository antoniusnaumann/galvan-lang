# Async Functions

> [!WARNING]
> **Not implemented yet.** `async fn`, `.await`, and async `main` generation
> do not transpile yet. This page describes the intended design.

Async functions are declared with `async fn` and awaited with `.await`.
Futures compose with the same error operators as synchronous code — awaiting
a fallible Rust call and propagating its error is `.await!`:

```galvan
async fn main() -> ! {
    let response = client.get("https://example.com").send().await!
    print response.text().await!
}
```

An async `main` uses the default async runtime without any attribute or
runtime setup in Galvan code.

> [!NOTE]
> In the Rust target, Galvan's default async runtime is Tokio: an async
> Galvan `main` will generate its Rust entry point on the Tokio runtime,
> without a `#[tokio::main]` annotation appearing in user code.

The design intentionally adds no new concepts: `async`/`.await` mirror Rust,
and error propagation stays postfix `!`, so `….await!` reads as "await, then
propagate failure".
