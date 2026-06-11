//! Service container for lazy initialization of dependencies.

use crate::ChatAppConfig;
use std::sync::Arc;

#[cfg(feature = "cli")]
use {
    botticelli_error::{ChatError, ChatErrorKind, ChatResult},
    tokio::sync::OnceCell,
    tracing::{debug, info, instrument},
};

#[cfg(feature = "cli")]
use {
    botticelli_database::PostgresNarrativeRepository,
    botticelli_storage::FileSystemStorage,
    diesel::PgConnection,
    diesel::r2d2::{ConnectionManager, Pool},
};

/// Container for lazily-initialized services.
///
/// Services are initialized on first use, allowing the chat interface
/// to start quickly and only connect to dependencies when needed.
pub struct ServiceContainer {
    config: Arc<ChatAppConfig>,

    #[cfg(feature = "cli")]
    db_pool: OnceCell<Pool<ConnectionManager<PgConnection>>>,

    #[cfg(feature = "cli")]
    narrative_repo: OnceCell<PostgresNarrativeRepository>,

    #[cfg(feature = "cli")]
    llm_provider: OnceCell<Arc<dyn botticelli_interface::BotticelliDriver>>,
}

impl ServiceContainer {
    /// Create a new service container with configuration.
    pub fn new(config: ChatAppConfig) -> Self {
        Self {
            config: Arc::new(config),

            #[cfg(feature = "cli")]
            db_pool: OnceCell::new(),

            #[cfg(feature = "cli")]
            narrative_repo: OnceCell::new(),

            #[cfg(feature = "cli")]
            llm_provider: OnceCell::new(),
        }
    }

    /// Get reference to configuration.
    pub fn config(&self) -> &ChatAppConfig {
        &self.config
    }

    #[cfg(feature = "cli")]
    /// Get or initialize database connection pool.
    ///
    /// The pool is created lazily on first access.
    #[instrument(skip(self))]
    pub async fn db_pool(&self) -> ChatResult<&Pool<ConnectionManager<PgConnection>>> {
        self.db_pool
            .get_or_try_init(|| async {
                info!("Initializing database connection pool");
                self.init_db_pool()
            })
            .await
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    fn init_db_pool(&self) -> ChatResult<Pool<ConnectionManager<PgConnection>>> {
        // DATABASE_URL should be set via .env file
        let db_url = self.config.postgres().database_url();

        debug!(url = %db_url, "Creating database connection pool");

        let manager = ConnectionManager::<PgConnection>::new(db_url);

        Pool::builder().max_size(10).build(manager).map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to create database pool: {}",
                e
            )))
        })
    }

    #[cfg(feature = "cli")]
    /// Check if database pool is initialized.
    pub fn is_db_initialized(&self) -> bool {
        self.db_pool.initialized()
    }

    #[cfg(feature = "cli")]
    /// Get or initialize narrative repository.
    ///
    /// The repository is created lazily on first access.
    #[instrument(skip(self))]
    pub async fn narrative_repository(&self) -> ChatResult<&PostgresNarrativeRepository> {
        self.narrative_repo
            .get_or_try_init(|| async {
                info!("Initializing narrative repository");
                self.init_narrative_repository().await
            })
            .await
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    async fn init_narrative_repository(&self) -> ChatResult<PostgresNarrativeRepository> {
        use diesel::prelude::*;

        // DATABASE_URL should be set via .env file
        let db_url = self.config.postgres().database_url();

        debug!(url = %db_url, "Creating narrative repository connection");

        // Create a dedicated connection for the repository
        let conn = PgConnection::establish(&db_url).map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to establish database connection: {}",
                e
            )))
        })?;

        // Create storage backend (for media)
        let storage_path = if self.config.environment().mode() == &crate::EnvironmentMode::Test {
            // Use temp directory for tests
            std::env::temp_dir().join("botticelli_test/media")
        } else {
            // Production path
            std::path::PathBuf::from("/var/botticelli/media")
        };

        let storage = std::sync::Arc::new(FileSystemStorage::new(storage_path).map_err(|e| {
            ChatError::new(ChatErrorKind::IoError(format!(
                "Failed to create storage: {}",
                e
            )))
        })?);

        let repo = PostgresNarrativeRepository::new(conn, storage);

        info!("Narrative repository initialized");
        Ok(repo)
    }

    #[cfg(feature = "cli")]
    /// Check if narrative repository is initialized.
    pub fn is_narrative_repo_initialized(&self) -> bool {
        self.narrative_repo.initialized()
    }

    #[cfg(feature = "cli")]
    /// Get or initialize LLM provider.
    ///
    /// The provider is created lazily on first access based on config.
    #[instrument(skip(self))]
    pub async fn llm_provider(
        &self,
    ) -> ChatResult<&Arc<dyn botticelli_interface::BotticelliDriver>> {
        self.llm_provider
            .get_or_try_init(|| async {
                info!("Initializing LLM provider");
                self.init_llm_provider()
            })
            .await
    }

    #[cfg(feature = "cli")]
    /// Get or initialize LLM provider with tool calling support.
    ///
    /// Returns provider cast as ToolCalling trait for MCP integration.
    /// Since GeminiClient (our default) implements ToolCalling, this is safe.
    #[instrument(skip(self))]
    pub async fn llm_provider_with_tools(
        &self,
    ) -> ChatResult<Arc<dyn botticelli_interface::ToolCalling>> {
        // Ensure base provider is initialized
        let _ = self.llm_provider().await?;

        // Create new client instance that we can cast to ToolCalling
        // This is necessary because we can't downcast trait objects
        let model_id = *self.config.chat().initial_model();
        self.create_tool_calling_client(model_id)
    }

    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    fn init_llm_provider(&self) -> ChatResult<Arc<dyn botticelli_interface::BotticelliDriver>> {
        let initial_model = *self.config.chat().initial_model();

        debug!(model = ?initial_model, "Initializing LLM provider");

        // Create initial client - fallback is handled at the executor level
        // using ChatSession and ModelSelector
        let client = self.create_client_for_model(initial_model)?;

        Ok(client)
    }

    /// Create a client for the specified model ID.
    ///
    /// This is used both for initial provider setup and for fallback scenarios.
    /// Supports creating clients for different model families dynamically.
    ///
    /// # Available with the `cli` feature
    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    pub fn create_client_for_model(
        &self,
        model_id: botticelli_models::ModelId,
    ) -> ChatResult<Arc<dyn botticelli_interface::BotticelliDriver>> {
        use botticelli_models::{GeminiClient, GroqDriver, ModelId};

        debug!(model = ?model_id, "Creating client for model");

        match model_id {
            ModelId::Groq(model) => {
                // GroqDriver::new() reads from GROQ_API_KEY environment variable
                let client = GroqDriver::new(model.to_string()).map_err(|e| {
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to create Groq client: {}",
                        e
                    )))
                })?;

                info!(model = ?model_id, "Groq client created successfully");
                Ok(Arc::new(client))
            }
            ModelId::Gemini(_model) => {
                // GeminiClient::new() reads from GEMINI_API_KEY environment variable
                // and uses gemini-2.5-flash as the default model
                let client = GeminiClient::new().map_err(|e| {
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to create Gemini client: {}",
                        e
                    )))
                })?;

                info!(model = ?model_id, "Gemini client created successfully");
                Ok(Arc::new(client))
            }
        }
    }

    /// Create a tool-calling client for the specified model ID.
    ///
    /// Returns the client cast as ToolCalling trait for MCP integration.
    ///
    /// # Available with the `cli` feature
    #[cfg(feature = "cli")]
    #[instrument(skip(self))]
    pub fn create_tool_calling_client(
        &self,
        model_id: botticelli_models::ModelId,
    ) -> ChatResult<Arc<dyn botticelli_interface::ToolCalling>> {
        use botticelli_models::{GeminiClient, GroqDriver, ModelId};

        debug!(model = ?model_id, "Creating tool-calling client for model");

        match model_id {
            ModelId::Groq(model) => {
                let client = GroqDriver::new(model.to_string()).map_err(|e| {
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to create Groq client: {}",
                        e
                    )))
                })?;

                info!(model = ?model_id, "Groq tool-calling client created");
                Ok(Arc::new(client) as Arc<dyn botticelli_interface::ToolCalling>)
            }
            ModelId::Gemini(_model) => {
                let client = GeminiClient::new().map_err(|e| {
                    ChatError::new(ChatErrorKind::ExecutionFailed(format!(
                        "Failed to create Gemini client: {}",
                        e
                    )))
                })?;

                info!(model = ?model_id, "Gemini tool-calling client created");
                Ok(Arc::new(client) as Arc<dyn botticelli_interface::ToolCalling>)
            }
        }
    }
}

impl std::fmt::Debug for ServiceContainer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceContainer")
            .field("config", &"<ChatAppConfig>")
            .finish()
    }
}
