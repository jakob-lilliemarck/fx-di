use crate::error::{InvokeResult, ProviderError, ProviderResult};
use futures::future::BoxFuture;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use tracing::instrument;

// ── Provider type ─────────────────────────────────────────────────────────────

type Provider = Box<
    dyn for<'a> FnOnce(
            &'a mut Container,
        ) -> BoxFuture<'a, Result<Box<dyn Any + Send + Sync>, ProviderError>>
        + Send
        + Sync,
>;

// ── Invokeable type ─────────────────────────────────────────────────────────────
type Invokeable =
    Box<dyn for<'a> FnOnce(&'a mut Container) -> BoxFuture<'a, InvokeResult> + Send + Sync>;

// ── Container ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct ResolutionEntry {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
}

/// A dependency injection container keyed by type.
pub struct Container {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    providers: HashMap<TypeId, Provider>,
    invokables: Vec<Invokeable>,
    resolution_path: Vec<ResolutionEntry>,
}

impl Container {
    /// Creates an empty container.
    #[instrument(level = "debug", skip_all)]
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            providers: HashMap::new(),
            invokables: Vec::new(),
            resolution_path: Vec::new(),
        }
    }

    /// Registers an async provider for `T`.
    ///
    /// The provider may resolve dependencies with `container.get::<Dep>().await?`.
    #[instrument(level = "debug", skip_all, fields(type_name))]
    pub fn provide<T, F>(&mut self, f: F)
    where
        T: Any + Send + Sync + Clone,
        F: for<'a> FnOnce(&'a mut Container) -> BoxFuture<'a, ProviderResult<T>>
            + Send
            + Sync
            + 'static,
    {
        let type_name = std::any::type_name::<T>();
        tracing::Span::current().record("type_name", type_name);
        tracing::debug!("fx_di::Container::provide");

        self.providers.insert(
            TypeId::of::<T>(),
            Box::new(move |c| {
                Box::pin(async move {
                    f(c).await
                        .map(|v| Box::new(v) as Box<dyn Any + Send + Sync>)
                }) as BoxFuture<'_, _>
            }),
        );
    }

    /// Resolves `T`, building it via its provider on first use and caching the result.
    ///
    /// Returns [`ProviderError::NoProvider`] if `T` is unregistered, and
    /// [`ProviderError::CycleDetected`] if resolving `T` depends on itself.
    #[instrument(level = "debug", skip(self), fields(type_name))]
    pub async fn get<T: Any + Send + Sync + Clone>(&mut self) -> Result<T, ProviderError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        tracing::Span::current().record("type_name", type_name);
        tracing::debug!(
            message = "fx_di::Container::get",
            resolution_path = ?self.resolution_path
        );

        // guard against cycles
        if self
            .resolution_path
            .iter()
            .any(|entry| entry.type_id == type_id)
        {
            return Err(ProviderError::cycle(&self.resolution_path, type_name));
        }

        // push the current type to the resolution path
        self.resolution_path
            .push(ResolutionEntry { type_id, type_name });

        // try to resolve the type
        let result = self.resolve::<T>().await;

        // pop from the resolution stack
        self.resolution_path.pop();

        result
    }

    /// Resolve a type, calling its provider if it has not yet been constructed.
    #[instrument(level = "debug", skip(self), fields(type_name))]
    async fn resolve<T: Any + Send + Sync + Clone>(&mut self) -> Result<T, ProviderError> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        if !self.values.contains_key(&type_id) {
            let provider = self
                .providers
                .remove(&type_id)
                .ok_or_else(|| ProviderError::NoProvider { type_name, type_id })?;

            let value = provider(self).await?;

            self.values.insert(type_id, value);
        }

        self.values
            .get(&type_id)
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
            .ok_or_else(|| ProviderError::NoValue { type_name })
    }

    /// Registers an async callback to run when [`Container::invoke`] is called.
    #[instrument(level = "debug", skip_all)]
    pub fn invokable<F>(&mut self, f: F)
    where
        F: for<'a> FnOnce(&'a mut Container) -> BoxFuture<'a, InvokeResult> + Send + Sync + 'static,
    {
        tracing::debug!(message = "fx_di::Container::invokable");

        self.invokables.push(Box::new(f))
    }

    /// Runs all registered invokables in order, then drains the queue.
    ///
    /// Stops at the first error.
    #[instrument(level = "debug", skip_all)]
    pub async fn invoke(&mut self) -> InvokeResult {
        tracing::debug!(message = "fx_di::Container::invoke");

        let invokables = std::mem::take(&mut self.invokables);
        for invokable in invokables {
            invokable(self).await?;
        }
        Ok(())
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}
