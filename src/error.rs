use std::any::TypeId;

use tracing::instrument;

use crate::container::ResolutionEntry;

// ── Errors ────────────────────────────────────────────────────────────────────

/// Error returned by [`Container::get`](crate::Container::get).
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// No provider was registered for the requested type.
    #[error("No provider registered for {type_name} with id {type_id:?}")]
    NoProvider {
        type_name: &'static str,
        type_id: TypeId,
    },

    /// The requested type depends on itself.
    #[error("Cycle detected:\n{0}")]
    CycleDetected(Cycle),

    /// A resolved value could not be downcast to the requested type.
    #[error("No value for: {type_name}")]
    NoValue { type_name: &'static str },

    /// A provider returned an error.
    #[error("Could not provide: {type_name}: {error}")]
    ProvideError {
        type_name: &'static str,
        #[source]
        error: Box<dyn std::error::Error + Sync + Send>,
    },
}

impl ProviderError {
    #[instrument(level = "debug")]
    pub(crate) fn cycle(entries: &[ResolutionEntry], recurring_type: &'static str) -> Self {
        Self::CycleDetected(Cycle {
            entries: entries.to_vec(),
            recurring_type,
        })
    }

    /// Wraps `error` into [`ProviderError::ProvideError`] for type `T`.
    #[instrument(level = "debug")]
    pub fn new<T, E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::ProvideError {
            type_name: std::any::type_name::<T>(),
            error: Box::new(error),
        }
    }
}

/// `Result<T, ProviderError>`.
pub type ProviderResult<T> = Result<T, ProviderError>;

/// A dependency cycle: the resolution path and the recurring type.
pub struct Cycle {
    entries: Vec<ResolutionEntry>,
    recurring_type: &'static str,
}

impl std::fmt::Display for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let culprit = self.entries.last().map(|e| &e.type_name);
        for entry in self.entries.iter() {
            if entry.type_name == self.recurring_type {
                writeln!(f, "\t{} ← first occurrence", entry.type_name)?;
            } else if Some(&entry.type_name) == culprit {
                writeln!(
                    f,
                    "\t{} ← provider depends on {}",
                    entry.type_name, self.recurring_type
                )?;
            } else {
                writeln!(f, "\t{}", entry.type_name)?;
            }
        }
        writeln!(f, "\t{}", self.recurring_type)
    }
}

impl std::fmt::Debug for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// Error returned by [`Container::invoke`](crate::Container::invoke).
#[derive(thiserror::Error, Debug)]
pub enum InvokeError {
    /// A provider failed while resolving inside an invokable.
    #[error("provide error: {0}")]
    Provide(#[from] ProviderError),

    /// An invokable returned an error.
    #[error("Could not invoke: {error}")]
    Invoke {
        #[source]
        error: Box<dyn std::error::Error + Sync + Send>,
    },
}

impl InvokeError {
    /// Wraps `error` into [`InvokeError::Invoke`].
    #[instrument(level = "debug")]
    pub fn new<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Invoke {
            error: Box::new(error),
        }
    }
}

/// `Result<(), InvokeError>`.
pub type InvokeResult = Result<(), InvokeError>;
