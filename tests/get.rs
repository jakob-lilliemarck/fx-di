use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use di::{Container, ProviderError, ProviderResult};
use futures::future::BoxFuture;

mod common;
use common::{
    A, C, D, E, F, counter_provider, provide_a, provide_b, provide_c, provide_d, provide_e,
    provide_f,
};

#[derive(Clone)]
struct Boom;

fn failing_provider(_: &mut Container) -> BoxFuture<'_, ProviderResult<Boom>> {
    Box::pin(async { Err(ProviderError::new::<Boom, _>(std::io::Error::other("boom"))) })
}

#[test]
fn it_constructs_a_dag() -> Result<(), ProviderError> {
    let mut container = Container::new();

    container.provide(provide_a);
    container.provide(provide_b);
    container.provide(provide_c);

    futures::executor::block_on(container.get::<Arc<C>>())?;

    Ok(())
}

#[test]
fn it_detects_cycles_and_provides_friendly_errors() {
    let mut container = Container::new();

    container.provide::<D, _>(provide_d);
    container.provide::<E, _>(provide_e);
    container.provide::<F, _>(provide_f);

    let result = futures::executor::block_on(container.get::<E>());

    match result {
        Err(ProviderError::CycleDetected(cycle)) => {
            assert!(cycle.to_string().contains("first occurrence"));
        }
        Err(other) => panic!("expected CycleDetected, got: {other}"),
        Ok(_) => panic!("expected error, got Ok"),
    }
}

#[test]
fn it_errors_when_provider_missing() {
    let mut container = Container::new();

    #[derive(Clone)]
    struct Unregistered;

    let result = futures::executor::block_on(container.get::<Unregistered>());

    assert!(matches!(result, Err(ProviderError::NoProvider { .. })));
}

#[test]
fn it_caches_resolved_values() {
    let mut container = Container::new();
    let counter = Arc::new(AtomicUsize::new(0));

    container.provide(counter_provider(counter.clone(), A));

    futures::executor::block_on(container.get::<A>()).unwrap();
    futures::executor::block_on(container.get::<A>()).unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn it_propagates_provider_errors() {
    let mut container = Container::new();

    container.provide(failing_provider);

    let result = futures::executor::block_on(container.get::<Boom>());

    match &result {
        Err(err) => {
            assert!(matches!(err, ProviderError::ProvideError { .. }));
            let source = err.source().expect("expected source");
            assert_eq!(source.to_string(), "boom");
        }
        Ok(_) => panic!("expected error, got Ok"),
    }
}
