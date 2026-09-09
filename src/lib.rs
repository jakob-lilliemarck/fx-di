//! Async, type-keyed dependency injection.
//!
//! Register a provider for each type and resolve the graph on demand.
//! Providers may depend on other types; cycles are detected at resolve time.
//!
//! # Example
//!
//! ```
//! use di::Container;
//! use futures::future::BoxFuture;
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct A;
//!
//! #[derive(Clone)]
//! struct B {
//!     _a: A,
//! }
//!
//! fn provide_a(_: &mut Container) -> BoxFuture<'_, di::ProviderResult<A>> {
//!     Box::pin(async { Ok(A) })
//! }
//!
//! fn provide_b(c: &mut Container) -> BoxFuture<'_, di::ProviderResult<Arc<B>>> {
//!     Box::pin(async {
//!         let a = c.get::<A>().await?;
//!         Ok(Arc::new(B { _a: a }))
//!     })
//! }
//!
//! # fn main() {
//! let mut container = Container::new();
//! container.provide(provide_a);
//! container.provide(provide_b);
//! futures::executor::block_on(container.get::<Arc<B>>()).unwrap();
//! # }
//! ```

mod container;
mod error;

pub use container::Container;
pub use error::{Cycle, InvokeError, InvokeResult, ProviderError, ProviderResult};
