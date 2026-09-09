# di

A minimal, async, type-keyed dependency injection container.

## What it does

`di` wires up an object graph for you. You register a provider for each type,
and the container resolves them in the right order on demand.

- **Automatic wiring** — a provider can ask for its dependencies with
  `container.get::<Dep>().await?`; the container builds the graph for you.
- **Cycle detection** — a dependency cycle fails with a readable error
  (including the resolution path) instead of hanging.
- **Cached** — each type is built once and reused.

## Usage

Add it:

```sh
cargo add di futures
```

Providers return `futures::future::BoxFuture`, so you'll want `futures` too.

```rust
use di::Container;
use futures::future::BoxFuture;
use std::sync::Arc;

#[derive(Clone)]
struct A;

#[derive(Clone)]
struct B {
    _a: A,
}

fn provide_a(_: &mut Container) -> BoxFuture<'_, di::ProviderResult<A>> {
    Box::pin(async { Ok(A) })
}

fn provide_b(c: &mut Container) -> BoxFuture<'_, di::ProviderResult<Arc<B>>> {
    Box::pin(async {
        let a = c.get::<A>().await?;
        Ok(Arc::new(B { _a: a }))
    })
}

fn main() {
    let mut container = Container::new();

    container.provide(provide_a);
    container.provide(provide_b);

    let b = futures::executor::block_on(container.get::<Arc<B>>()).unwrap();
}
```

`di` is runtime-agnostic — use tokio, async-std, or any executor (the example
uses `futures::executor::block_on`).

### API

| Method | What it does |
| --- | --- |
| `Container::new()` | Create an empty container. |
| `Container::provide(f)` | Register an async provider for a type `T`. |
| `Container::get::<T>()` | Resolve a type (async). Cached after first call. |
| `Container::invokable(f)` | Register a callback to run later. |
| `Container::invoke()` | Run all registered invokables, in order (async). |

### Errors

`get` returns `Result<T, ProviderError>`; `invoke` returns `InvokeResult`.
Both implement `std::error::Error` with readable messages.
