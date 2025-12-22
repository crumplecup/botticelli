//! MCP-based chat host implementation.

use botticelli_core::{Input, Message, MessageBuilder, Role, ToolDefinition};
use botticelli_error::{ChatError, ChatErrorKind, ChatResult};
use botticelli_interface::{ChatHost, ChatMessage, ToolCalling};
use botticelli_mcp_client::McpHost;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::{ConversationLoop, ToolCallHandler};

/// Chat host that integrates LLM with MCP tools.
#[derive(derive_getters::Getters)]
pub struct McpChatHost {
    /// MCP host for tool execution
    mcp_host: Arc<Mutex<McpHost>>,
    
    /// Tool call handler
    tool_handler: Arc<RwLock<ToolCallHandler>>,
    
    /// Conversation loop
    conversation_loop: ConversationLoop,
    
    /// LLM provider with tool calling capability
    llm_provider: Arc<dyn ToolCalling + Send + Sync>,
    
    /// Conversation history
    conversation: Vec<Message>,
}

impl McpChatHost {
    /// Create a new MCP chat host.
    pub fn new(
        mcp_host: Arc<Mutex<McpHost>>,
        llm_provider: Arc<dyn ToolCalling + Send + Sync>,
    ) -> Self {
        let tool_handler = Arc::new(RwLock::new(ToolCallHandler::new(mcp_host.clone())));
        let conversation_loop = ConversationLoop::new(tool_handler.clone());
        
        Self {
            mcp_host,
            tool_handler,
            conversation_loop,
            llm_provider,
            conversation: Vec::new(),
        }
    }
}

impl ChatHost for McpChatHost {
    #[tracing::instrument(skip(self), fields(message_len = user_message.len()))]
    fn send_message(&mut self, user_message: String) -> ChatResult<String> {
        tracing::debug!("Sending user message");
        
        // Add user message to conversation
        let user_msg = MessageBuilder::default()
            .role(Role::User)
            .content(vec![Input::Text(user_message)])
            .build()
            .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Failed to build message: {}", e))))?;
        
        self.conversation.push(user_msg);
        
        // Get available tools
        let tools = self.available_tools()?;
        tracing::debug!(tool_count = tools.len(), "Got available tools");
        
        // Run conversation loop with tools
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Failed to create runtime: {}", e))))?;
        
        let messages = rt.block_on(async {
            self.conversation_loop
                .run_conversation(self.llm_provider.as_ref(), self.conversation.clone(), &tools)
                .await
        })?;
        
        // Update conversation with results
        self.conversation = messages;
        
        // Extract final assistant response
        let response = self.conversation
            .iter()
            .rev()
            .find(|m| m.role() == &Role::Assistant)
            .and_then(|m| m.content().first())
            .and_then(|output| {
                use botticelli_core::Output;
                if let Output::Text(text) = output {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "No response".to_string());
        
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    fn get_conversation(&self) -> ChatResult<Vec<ChatMessage>> {
        tracing::debug!(count = self.conversation.len(), "Getting conversation");
        
        let chat_messages = self.conversation
            .iter()
            .filter_map(|msg| {
                let role = match msg.role() {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                };
                
                use botticelli_core::Output;
                let content = msg.content()
                    .iter()
                    .filter_map(|output| {
                        if let Output::Text(text) = output {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                if !content.is_empty() {
                    Some(ChatMessage {
                        role: role.to_string(),
                        content,
                    })
                } else {
                    None
                }
            })
            .collect();
        
        Ok(chat_messages)
    }

    #[tracing::instrument(skip(self))]
    fn available_tools(&self) -> ChatResult<Vec<ToolDefinition>> {
        tracing::debug!("Getting available tools from MCP host");
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Failed to create runtime: {}", e))))?;
        
        let mcp_host = rt.block_on(self.mcp_host.lock());
        let tools = mcp_host.list_tools()
            .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Failed to list tools: {}", e))))?;
        
        tracing::debug!(tool_count = tools.len(), tool_names = ?tools.iter().map(|t| t.name()).collect::<Vec<_>>(), "Retrieved tools");
        
        Ok(tools)
    }

    #[tracing::instrument(skip(self, arguments))]
    fn execute_tool(&mut self, name: &str, arguments: serde_json::Value) -> ChatResult<serde_json::Value> {
        tracing::debug!(tool_name = name, "Executing tool");
        
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Failed to create runtime: {}", e))))?;
        
        let result = rt.block_on(async {
            let mut mcp_host = self.mcp_host.lock().await;
            mcp_host.call_tool(name, arguments).await
        })
        .map_err(|e| ChatError::new(ChatErrorKind::ExecutionFailed(format!("Tool execution failed: {}", e))))?;
        
        Ok(result)
    }
}
