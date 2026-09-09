use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use fx_di::Container;

mod common;
use common::{A, counter_provider};

#[test]
fn it_registers_providers_lazily() {
    let mut container = Container::new();
    let counter = Arc::new(AtomicUsize::new(0));

    container.provide(counter_provider(counter.clone(), A));

    assert_eq!(counter.load(Ordering::SeqCst), 0);
}

#[test]
fn it_overwrites_existing_provider() {
    let mut container = Container::new();

    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));

    container.provide(counter_provider(first.clone(), A));
    container.provide(counter_provider(second.clone(), A));

    futures::executor::block_on(container.get::<A>()).unwrap();

    assert_eq!(first.load(Ordering::SeqCst), 0);
    assert_eq!(second.load(Ordering::SeqCst), 1);
}
