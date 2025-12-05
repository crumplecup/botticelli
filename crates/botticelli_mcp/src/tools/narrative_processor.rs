//! Processor integration for MCP narrative execution.
//!
//! This module provides MCP-compatible wrappers for narrative processors,
//! allowing extracted data to be returned to the MCP client.

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_interface::ProcessorTrait;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use botticelli_narrative::ProcessorContext;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use serde_json::Value;

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use std::sync::{Arc, Mutex};

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
use tracing::{debug, instrument};

/// Collects processor outputs for MCP response.
///
/// This processor wraps other processors and collects their outputs
/// so they can be returned to the MCP client along with the narrative response.
#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
#[derive(Clone)]
pub struct McpProcessorCollector {
    /// Collected outputs from processors
    outputs: Arc<Mutex<Vec<(String, Value)>>>,
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl McpProcessorCollector {
    /// Create a new processor collector.
    pub fn new() -> Self {
        Self {
            outputs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get collected processor outputs.
    pub fn outputs(&self) -> Vec<(String, Value)> {
        self.outputs.lock().unwrap().clone()
    }

    /// Clear collected outputs.
    pub fn clear(&self) {
        self.outputs.lock().unwrap().clear();
    }
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
impl Default for McpProcessorCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(
    feature = "gemini",
    feature = "anthropic",
    feature = "ollama",
    feature = "huggingface",
    feature = "groq"
))]
#[async_trait::async_trait]
impl ProcessorTrait for McpProcessorCollector {
    fn name(&self) -> &str {
        "mcp_collector"
    }

    #[instrument(skip(self, _context), fields(processor = "mcp_collector"))]
    async fn process(
        &self,
        _context: &ProcessorContext,
    ) -> Result<Option<Value>, Box<dyn std::error::Error + Send + Sync>> {
        debug!("MCP processor collector called (passthrough)");
        // This processor doesn't transform data, it just collects from others
        Ok(None)
    }
}
