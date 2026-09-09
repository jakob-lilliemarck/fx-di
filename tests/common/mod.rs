#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::future::BoxFuture;
use fx_di::{Container, InvokeResult, ProviderResult};

#[derive(Clone)]
pub struct A;

#[derive(Clone)]
pub struct B {
    _a: A,
}

#[derive(Clone)]
pub struct C {
    _a: A,
    _b: Arc<B>,
}

#[derive(Clone)]
pub struct D;

#[derive(Clone)]
pub struct E;

#[derive(Clone)]
pub struct F;

pub fn provide_a(_: &mut Container) -> BoxFuture<'_, ProviderResult<A>> {
    Box::pin(async { Ok(A) })
}

pub fn provide_b(container: &mut Container) -> BoxFuture<'_, ProviderResult<Arc<B>>> {
    Box::pin(async {
        let a = container.get::<A>().await?;
        Ok(Arc::new(B { _a: a }))
    })
}

pub fn provide_c(container: &mut Container) -> BoxFuture<'_, ProviderResult<Arc<C>>> {
    Box::pin(async {
        let a = container.get::<A>().await?;
        let b = container.get::<Arc<B>>().await?;
        Ok(Arc::new(C { _a: a, _b: b }))
    })
}

pub fn provide_d(c: &mut Container) -> BoxFuture<'_, ProviderResult<D>> {
    Box::pin(async {
        c.get::<E>().await?;
        Ok(D)
    })
}

pub fn provide_e(c: &mut Container) -> BoxFuture<'_, ProviderResult<E>> {
    Box::pin(async {
        c.get::<F>().await?;
        Ok(E)
    })
}

pub fn provide_f(c: &mut Container) -> BoxFuture<'_, ProviderResult<F>> {
    Box::pin(async {
        c.get::<D>().await?;
        Ok(F)
    })
}

pub fn counter_provider<T>(
    counter: Arc<AtomicUsize>,
    value: T,
) -> impl for<'a> FnOnce(&'a mut Container) -> BoxFuture<'a, ProviderResult<T>>
where
    T: Clone + Send + Sync + 'static,
{
    move |_c| {
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(value.clone())
        }) as BoxFuture<'_, _>
    }
}

pub fn counter_invokable(
    counter: Arc<AtomicUsize>,
) -> impl for<'a> FnOnce(&'a mut Container) -> BoxFuture<'a, InvokeResult> {
    move |_c| {
        Box::pin(async move {
            counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }) as BoxFuture<'_, _>
    }
}
