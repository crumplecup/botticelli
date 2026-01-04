//! Error conversion macros for reducing boilerplate.

/// Creates a bridge From implementation for external errors.
///
/// This macro generates the conversion chain:
/// `ExternalError → WrapperError → ErrorKind`
///
/// The wrapper error type must already have `impl From<ExternalError>` defined.
///
/// # Examples
///
/// ```ignore
/// // Given: impl From<reqwest::Error> for HttpError
/// bridge_error!(reqwest::Error => HttpError => BotticelliErrorKind);
///
/// // Generates:
/// impl From<reqwest::Error> for BotticelliErrorKind {
///     #[track_caller]
///     fn from(err: reqwest::Error) -> Self {
///         HttpError::from(err).into()
///     }
/// }
/// ```
#[macro_export]
macro_rules! bridge_error {
    ($external:ty => $wrapper:ty => $kind:ty) => {
        impl From<$external> for $kind {
            #[track_caller]
            fn from(err: $external) -> Self {
                <$wrapper>::from(err).into()
            }
        }
    };
}

/// Creates a From implementation with tracing for the top-level Error type.
///
/// This macro generates the full conversion chain:
/// `SourceError → ErrorKind → Error`
///
/// The conversion automatically logs the error at the ERROR level with the
/// error_kind field for observability.
///
/// # Examples
///
/// ```ignore
/// error_from!(HttpError => BotticelliError);
///
/// // Generates:
/// impl From<HttpError> for BotticelliError {
///     #[track_caller]
///     fn from(err: HttpError) -> Self {
///         let kind = BotticelliErrorKind::from(err);
///         tracing::error!(error_kind = %kind, "Error created");
///         Self(Box::new(kind))
///     }
/// }
/// ```
#[macro_export]
macro_rules! error_from {
    ($source:ty => $error:ty) => {
        impl From<$source> for $error {
            #[track_caller]
            fn from(err: $source) -> Self {
                let kind = err.into();
                tracing::error!(error_kind = %kind, "Error created");
                Self(Box::new(kind))
            }
        }
    };
}

/// Creates a From implementation from ErrorKind to Error wrapper.
///
/// This macro generates the conversion from an ErrorKind enum to its
/// corresponding Error wrapper struct with automatic location tracking.
///
/// # Examples
///
/// ```ignore
/// impl_error_from_kind!(TuiErrorKind => TuiError);
///
/// // Generates:
/// impl From<TuiErrorKind> for TuiError {
///     #[track_caller]
///     fn from(kind: TuiErrorKind) -> Self {
///         Self::new(kind)
///     }
/// }
/// ```
#[macro_export]
macro_rules! impl_error_from_kind {
    ($kind:ty => $error:ty) => {
        impl From<$kind> for $error {
            #[track_caller]
            fn from(kind: $kind) -> Self {
                Self::new(kind)
            }
        }
    };
}

/// Creates a chained conversion from specific ErrorKind to parent ErrorKind variant.
///
/// This macro wraps a specific ErrorKind in a parent ErrorKind variant directly.
///
/// # Examples
///
/// ```ignore
/// chain_error_kind!(OllamaErrorKind => ModelsErrorKind, Ollama);
///
/// // Generates:
/// impl From<OllamaErrorKind> for ModelsErrorKind {
///     #[track_caller]
///     fn from(kind: OllamaErrorKind) -> Self {
///         ModelsErrorKind::Ollama(kind)
///     }
/// }
/// ```
#[macro_export]
macro_rules! chain_error_kind {
    ($kind:ty => $parent_kind:path, $variant:ident) => {
        impl From<$kind> for $parent_kind {
            #[track_caller]
            fn from(kind: $kind) -> Self {
                <$parent_kind>::$variant(kind)
            }
        }
    };
}
