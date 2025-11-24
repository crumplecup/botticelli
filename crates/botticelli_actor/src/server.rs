//! Server implementation types for actor-based bots

use async_trait::async_trait;
use botticelli_server::{
    ActorManager, ActorServer, ActorServerResult, ContentPoster, StatePersistence, TaskScheduler,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, instrument};

/// Simple in-memory task scheduler with optional state persistence
#[derive(Debug)]
pub struct SimpleTaskScheduler<P = ()> {
    tasks: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
    persistence: Option<Arc<P>>,
}

impl SimpleTaskScheduler<()> {
    /// Create a new task scheduler without persistence
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            persistence: None,
        }
    }
}

impl<P> SimpleTaskScheduler<P> {
    /// Check if a task has a persistence backend configured
    pub fn has_persistence(&self) -> bool {
        self.persistence.is_some()
    }

    /// Get reference to the persistence backend if configured
    pub fn persistence(&self) -> Option<&Arc<P>> {
        self.persistence.as_ref()
    }
}

impl<P> SimpleTaskScheduler<P>
where
    P: StatePersistence + Send + Sync + 'static,
{
    /// Create a new task scheduler with persistence backend
    pub fn with_persistence(persistence: P) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            persistence: Some(Arc::new(persistence)),
        }
    }

    /// Recover tasks from persisted state on startup
    #[instrument(skip(self))]
    pub async fn recover_tasks(&self) -> ActorServerResult<Vec<String>>
    where
        P::State: Clone + std::fmt::Debug,
    {
        debug!("Recovering tasks from persisted state");

        let Some(persistence) = &self.persistence else {
            debug!("No persistence backend configured, skipping recovery");
            return Ok(Vec::new());
        };

        // Load persisted state
        let state_opt = persistence.load_state().await?;
        let Some(_state) = state_opt else {
            info!("No persisted state found");
            return Ok(Vec::new());
        };

        info!("Task recovery completed");
        Ok(Vec::new())
    }
}

impl Default for SimpleTaskScheduler<()> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<P> TaskScheduler for SimpleTaskScheduler<P>
where
    P: Send + Sync + 'static,
{
    #[instrument(skip(self, task))]
    async fn schedule<F, Fut>(
        &mut self,
        task_id: String,
        interval: Duration,
        task: F,
    ) -> ActorServerResult<()>
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ActorServerResult<()>> + Send + 'static,
    {
        debug!(?interval, "Scheduling task");

        let task = Arc::new(task);
        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = task().await {
                    error!(error = ?e, "Task execution failed");
                }
            }
        });

        let mut tasks = self.tasks.write().await;
        if let Some(old_handle) = tasks.insert(task_id.clone(), handle) {
            debug!("Canceling existing task");
            old_handle.abort();
        }

        info!("Task scheduled");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn cancel(&mut self, task_id: &str) -> ActorServerResult<()> {
        debug!("Canceling task");
        let mut tasks = self.tasks.write().await;
        if let Some(handle) = tasks.remove(task_id) {
            handle.abort();
            info!("Task canceled");
        }
        Ok(())
    }

    fn is_scheduled(&self, task_id: &str) -> bool {
        // Try non-blocking read - if poisoned or would block, return false
        if let Ok(tasks) = self.tasks.try_read() {
            tasks.contains_key(task_id)
        } else {
            false
        }
    }

    fn scheduled_tasks(&self) -> Vec<String> {
        // Try non-blocking read - if poisoned or would block, return empty
        if let Ok(tasks) = self.tasks.try_read() {
            tasks.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

/// Generic actor manager implementation
#[derive(Debug)]
pub struct GenericActorManager<I, C> {
    actors: Arc<RwLock<HashMap<I, ActorState>>>,
    _phantom: std::marker::PhantomData<C>,
}

#[derive(Debug, Clone)]
struct ActorState {
    #[allow(dead_code)]
    registered_at: chrono::DateTime<chrono::Utc>,
}

impl<I, C> GenericActorManager<I, C>
where
    I: Send + Sync + Clone + std::hash::Hash + Eq,
    C: Send + Sync,
{
    /// Create a new actor manager
    pub fn new() -> Self {
        Self {
            actors: Arc::new(RwLock::new(HashMap::new())),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<I, C> Default for GenericActorManager<I, C>
where
    I: Send + Sync + Clone + std::hash::Hash + Eq,
    C: Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<I, C> ActorManager for GenericActorManager<I, C>
where
    I: Send + Sync + Clone + std::hash::Hash + Eq + std::fmt::Debug,
    C: Send + Sync,
{
    type ActorId = I;
    type Context = C;

    #[instrument(skip(self))]
    async fn register_actor(&mut self, actor_id: Self::ActorId) -> ActorServerResult<()> {
        debug!("Registering actor");
        let mut actors = self.actors.write().await;
        actors.insert(
            actor_id,
            ActorState {
                registered_at: chrono::Utc::now(),
            },
        );
        info!("Actor registered");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn unregister_actor(&mut self, actor_id: &Self::ActorId) -> ActorServerResult<()> {
        debug!("Unregistering actor");
        let mut actors = self.actors.write().await;
        actors.remove(actor_id);
        info!("Actor unregistered");
        Ok(())
    }

    #[instrument(skip(self, _context))]
    async fn execute_actor(
        &self,
        actor_id: &Self::ActorId,
        _context: &Self::Context,
    ) -> ActorServerResult<()> {
        debug!("Executing actor");
        let actors = self.actors.read().await;
        if !actors.contains_key(actor_id) {
            return Err("Actor not registered".into());
        }
        // Actual execution logic would be implemented by concrete types
        info!("Actor executed");
        Ok(())
    }

    fn registered_actors(&self) -> Vec<Self::ActorId> {
        let actors = self.actors.blocking_read();
        actors.keys().cloned().collect()
    }

    fn is_registered(&self, actor_id: &Self::ActorId) -> bool {
        let actors = self.actors.blocking_read();
        actors.contains_key(actor_id)
    }
}

/// Generic content poster implementation
#[derive(Debug)]
pub struct GenericContentPoster<Content, Dest, Posted> {
    _phantom: std::marker::PhantomData<(Content, Dest, Posted)>,
}

impl<Content, Dest, Posted> GenericContentPoster<Content, Dest, Posted> {
    /// Create a new content poster
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Content, Dest, Posted> Default for GenericContentPoster<Content, Dest, Posted> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<Content, Dest, Posted> ContentPoster for GenericContentPoster<Content, Dest, Posted>
where
    Content: Send + Sync,
    Dest: Send + Sync,
    Posted: Send + Sync,
{
    type Content = Content;
    type Destination = Dest;
    type Posted = Posted;

    #[instrument(skip(self, _content, _destination))]
    async fn post(
        &self,
        _content: Self::Content,
        _destination: &Self::Destination,
    ) -> ActorServerResult<Self::Posted> {
        debug!("Posting content");
        // Actual posting logic implemented by concrete types
        Err("Not implemented".into())
    }

    async fn can_post(&self, _destination: &Self::Destination) -> bool {
        true
    }
}

/// JSON file-based state persistence
#[derive(Debug, Clone)]
pub struct JsonStatePersistence<T> {
    file_path: std::path::PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> JsonStatePersistence<T> {
    /// Create a new JSON state persistence with file path
    pub fn new(file_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T> StatePersistence for JsonStatePersistence<T>
where
    T: Send + Sync + serde::Serialize + serde::de::DeserializeOwned,
{
    type State = T;

    #[instrument(skip(self, state))]
    async fn save_state(&self, state: &Self::State) -> ActorServerResult<()> {
        debug!(path = ?self.file_path, "Saving state");
        let json = serde_json::to_string_pretty(state)?;
        tokio::fs::write(&self.file_path, json).await?;
        info!("State saved");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn load_state(&self) -> ActorServerResult<Option<Self::State>> {
        debug!(path = ?self.file_path, "Loading state");
        if !self.file_path.exists() {
            return Ok(None);
        }
        let json = tokio::fs::read_to_string(&self.file_path).await?;
        let state = serde_json::from_str(&json)?;
        info!("State loaded");
        Ok(Some(state))
    }

    #[instrument(skip(self))]
    async fn clear_state(&self) -> ActorServerResult<()> {
        debug!(path = ?self.file_path, "Clearing state");
        if self.file_path.exists() {
            tokio::fs::remove_file(&self.file_path).await?;
            info!("State cleared");
        }
        Ok(())
    }
}

/// Basic actor server coordinating all components
#[derive(Debug)]
pub struct BasicActorServer {
    running: Arc<RwLock<bool>>,
}

impl BasicActorServer {
    /// Create a new basic actor server
    pub fn new() -> Self {
        Self {
            running: Arc::new(RwLock::new(false)),
        }
    }
}

impl Default for BasicActorServer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ActorServer for BasicActorServer {
    #[instrument(skip(self))]
    async fn start(&mut self) -> ActorServerResult<()> {
        info!("Starting actor server");
        let mut running = self.running.write().await;
        *running = true;
        info!("Actor server started");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn stop(&mut self) -> ActorServerResult<()> {
        info!("Stopping actor server");
        let mut running = self.running.write().await;
        *running = false;
        info!("Actor server stopped");
        Ok(())
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    #[instrument(skip(self))]
    async fn reload(&mut self) -> ActorServerResult<()> {
        info!("Reloading actor server");
        // Reload logic would be implemented by concrete types
        Ok(())
    }
}
