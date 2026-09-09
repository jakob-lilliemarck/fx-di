use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::future::BoxFuture;
use fx_di::{Container, InvokeError, InvokeResult};

mod common;
use common::{A, counter_invokable, provide_a};

#[test]
fn it_runs_all_invokables() {
    let mut container = Container::new();
    let counter = Arc::new(AtomicUsize::new(0));

    container.invokable(counter_invokable(counter.clone()));
    container.invokable(counter_invokable(counter.clone()));

    futures::executor::block_on(container.invoke()).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[test]
fn it_invokes_lazily_and_drains() {
    let mut container = Container::new();
    let counter = Arc::new(AtomicUsize::new(0));

    container.invokable(counter_invokable(counter.clone()));
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    futures::executor::block_on(container.invoke()).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    futures::executor::block_on(container.invoke()).unwrap();
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn it_invokes_with_resolved_values() {
    let mut container = Container::new();

    container.provide(provide_a);
    container.invokable(|c| {
        Box::pin(async {
            let _a = c.get::<A>().await?;
            Ok(())
        }) as BoxFuture<'_, InvokeResult>
    });

    futures::executor::block_on(container.invoke()).unwrap();
}

#[test]
fn it_stops_on_first_error() {
    let mut container = Container::new();
    let counter = Arc::new(AtomicUsize::new(0));

    container.invokable(|_c| {
        Box::pin(async { Err(InvokeError::new(std::io::Error::other("boom"))) })
            as BoxFuture<'_, InvokeResult>
    });
    container.invokable(counter_invokable(counter.clone()));

    let result = futures::executor::block_on(container.invoke());

    assert!(matches!(result, Err(InvokeError::Invoke { .. })));
    assert_eq!(counter.load(Ordering::SeqCst), 0);
}
