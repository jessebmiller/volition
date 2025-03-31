# Streamlined Volition MCP Implementation Plan

## Overview

This project plan outlines a simplified approach to implement the Model Context Protocol (MCP) in Volition without concern for backward compatibility. The plan focuses on essential functionality first, with a clear path for future expansion.

## Background

The Model Context Protocol (MCP) is an open standard that standardizes how applications provide context to LLMs. It implements a client-server architecture where:

- **MCP Hosts**: Programs like Claude Desktop that want to access data through MCP
- **MCP Clients**: Protocol clients that maintain 1:1 connections with servers
- **MCP Servers**: Lightweight programs exposing specific capabilities

## Project Goals

1. Implement MCP client capabilities using the official Rust SDK
2. Implement core tools as MCP servers (filesystem and shell)
3. Implement file read/write as MCP resources
4. Add basic multi-provider strategy support

## Simplified Architecture

### 1. MCP-First Architecture

```
volition-agent-core/src/
├── mcp/
│   ├── client.rs        # Main MCP client implementation
│   ├── mod.rs           # Module exports
│   └── session.rs       # Session management
├── providers/           # LLM provider implementations
│   ├── mod.rs           # Provider registry & trait
│   ├── openai.rs        # OpenAI provider implementation 
│   ├── anthropic.rs     # Anthropic provider implementation
│   └── gemini.rs        # Gemini provider implementation
├── servers/             # MCP server implementations
│   ├── filesystem/      # File operations
│   └── shell/           # Shell command execution
├── strategies/          # Strategy implementations
│   ├── mod.rs
│   ├── complete_task.rs
│   └── plan_execute.rs  # Simple two-provider strategy
└── agent.rs             # Main agent using MCP
```

### 2. MCP Protocol Integration

Instead of adapting existing tools to MCP, we'll:
- Use the official Rust SDK (`rmcp` crate) for both client and server implementations
- Implement custom MCP servers in Rust for each functionality
- Connect directly to MCP servers via the Rust SDK's transport abstractions

### 3. Resource-Based File Operations

Files will be handled purely as resources:
- File reads via `resource/get`
- File writes via `resource/set`
- Directory listings via resources

## Implementation Phases

### Phase 1: MCP Foundation with Rust SDK (1 week)

1. Set up MCP client using the official Rust SDK (`rmcp`)
2. Implement basic MCP session management
3. Create MCP server launcher/manager
4. Establish communication patterns

**Key Components:**
- MCP client implementation using `rmcp` crate
- Server process management with Tokio
- Connection handling with the SDK's transport abstractions

### Phase 2: Core Servers Implementation (1 week)

1. Implement filesystem MCP server using the Rust SDK
2. Implement shell command MCP server using the Rust SDK
3. Implement Git operations server using the Rust SDK
4. Implement search functionality server using the Rust SDK

**Key Deliverables:**
- Standalone Rust MCP servers for core functionality
- Server configuration system
- User confirmation mechanisms for sensitive operations

**Example Server Implementation:**
```rust
// volition-servers/filesystem/src/main.rs
use anyhow::Result;
use rmcp::{Server, ServerBuilder, Tool, Resource};
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    // Create the server
    let mut server = ServerBuilder::new("filesystem-server", "1.0.0")
        .build();
    
    // Add file read tool
    server.add_tool(
        "read_file",
        |args: Value| async move {
            let path = args["path"].as_str().ok_or_else(|| anyhow!("Missing path argument"))?;
            let content = fs::read_to_string(path).await?;
            Ok(json!({ "content": content }))
        },
    );
    
    // Add file write tool
    server.add_tool(
        "write_file",
        |args: Value| async move {
            let path = args["path"].as_str().ok_or_else(|| anyhow!("Missing path argument"))?;
            let content = args["content"].as_str().ok_or_else(|| anyhow!("Missing content argument"))?;
            fs::write(path, content).await?;
            Ok(json!({ "success": true }))
        },
    );
    
    // Start the server using stdio transport
    server.serve_stdio().await?;
    
    Ok(())
}
```

### Phase 3: Minimal Multi-Provider Agent (1 week)

1. Implement basic provider switching mechanism
2. Create simple configuration for strategy-provider mapping
3. Add provider-specific message context management
4. Build plan/execute strategy with two providers

**Key Features:**
- Simple provider registry with direct mapping
- Basic provider switching between planning and execution
- Message history transformation between providers
- Minimal configuration system

```rust
// volition-agent-core/src/providers/mod.rs
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

// Simple provider trait
#[async_trait]
pub trait Provider: Send + Sync {
    async fn get_completion(&self, messages: Vec<Message>) -> Result<ApiResponse>;
    fn name(&self) -> &str;
}

// Provider registry
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
    default_provider: String,
}

impl ProviderRegistry {
    pub fn new(default_provider: String) -> Self {
        Self {
            providers: HashMap::new(),
            default_provider,
        }
    }

    pub fn register(&mut self, id: String, provider: Box<dyn Provider>) {
        self.providers.insert(id, provider);
    }

    pub fn get(&self, id: &str) -> Result<&dyn Provider> {
        self.providers.get(id)
            .map(|p| p.as_ref())
            .ok_or_else(|| anyhow!("Provider not found: {}", id))
    }

    pub fn default(&self) -> Result<&dyn Provider> {
        self.get(&self.default_provider)
    }
}

// Simple agent with provider switching
pub struct Agent {
    provider_registry: ProviderRegistry,
    current_provider_id: String,
}

impl Agent {
    pub fn new(provider_registry: ProviderRegistry) -> Self {
        let default_id = provider_registry.default_provider.clone();
        Self {
            provider_registry,
            current_provider_id: default_id,
        }
    }

    pub fn switch_provider(&mut self, provider_id: &str) -> Result<()> {
        // Check if provider exists
        self.provider_registry.get(provider_id)?;
        self.current_provider_id = provider_id.to_string();
        Ok(())
    }

    pub async fn get_completion(&self, messages: Vec<Message>) -> Result<ApiResponse> {
        let provider = self.provider_registry.get(&self.current_provider_id)?;
        provider.get_completion(messages).await
    }
}
```

**Simple Strategy Implementation:**
```rust
// volition-agent-core/src/strategies/plan_execute.rs
pub struct PlanExecuteStrategy {
    planning_provider: String,
    execution_provider: String,
}

impl PlanExecuteStrategy {
    pub fn new(planning_provider: String, execution_provider: String) -> Self {
        Self {
            planning_provider,
            execution_provider,
        }
    }
}

impl Strategy for PlanExecuteStrategy {
    async fn run(&mut self, agent: &mut Agent, state: &mut AgentState) -> Result<String> {
        // Planning phase
        agent.switch_provider(&self.planning_provider)?;
        
        // Use system prompt for planning
        let planning_messages = vec![
            Message::system("You are a planning assistant. Create a step-by-step plan."),
            Message::user(&state.initial_task),
        ];
        
        let plan_response = agent.get_completion(planning_messages).await?;
        let plan = plan_response.message.content.clone();
        
        // Execution phase
        agent.switch_provider(&self.execution_provider)?;
        
        // Pass the plan to the execution model
        let execution_messages = vec![
            Message::system("You are an execution assistant. Execute the given plan."),
            Message::user(format!("Plan:\n{}\n\nExecute this plan.", plan)),
        ];
        
        let execution_response = agent.get_completion(execution_messages).await?;
        
        Ok(execution_response.message.content)
    }
}
```

### Phase 4: Testing and Polish (1 week)

1. Integration tests for MCP interactions
2. End-to-end strategy tests
3. Command-line interface enhancements
4. Documentation and examples

## Code Examples

### 1. Simplified MCP Client with Rust SDK

```rust
// volition-agent-core/src/mcp/client.rs
use anyhow::{anyhow, Result};
use rmcp::{Client, ServiceExt, transport::TokioChildProcess};
use serde_json::Value;
use tokio::process::Command;

pub struct McpClient {
    server_command: String,
    server_args: Vec<String>,
    client: Option<Client>,
}

impl McpClient {
    pub async fn new(server_command: String, server_args: Vec<String>) -> Result<Self> {
        Ok(Self {
            server_command,
            server_args,
            client: None,
        })
    }
    
    pub async fn connect(&mut self) -> Result<()> {
        // Create the command for the MCP server
        let mut cmd = Command::new(&self.server_command);
        for arg in &self.server_args {
            cmd.arg(arg);
        }
        
        // Launch the server and connect to it using the Rust SDK
        let client = ().serve(TokioChildProcess::new(cmd)?)
            .await
            .map_err(|e| anyhow!("Failed to connect to MCP server: {}", e))?;
        
        self.client = Some(client);
        Ok(())
    }
    
    pub async fn list_tools(&self) -> Result<Vec<rmcp::Tool>> {
        let client = self.client.as_ref().ok_or(anyhow!("Not connected"))?;
        let tools = client.list_tools().await
            .map_err(|e| anyhow!("Failed to list tools: {}", e))?;
        Ok(tools)
    }
    
    pub async fn call_tool(&self, name: &str, args: Value) -> Result<Value> {
        let client = self.client.as_ref().ok_or(anyhow!("Not connected"))?;
        let result = client.call_tool(name, args).await
            .map_err(|e| anyhow!("Failed to call tool '{}': {}", name, e))?;
        Ok(result)
    }
    
    pub async fn get_resource(&self, uri: &str) -> Result<Value> {
        let client = self.client.as_ref().ok_or(anyhow!("Not connected"))?;
        let resource = client.get_resource(uri).await
            .map_err(|e| anyhow!("Failed to get resource '{}': {}", uri, e))?;
        Ok(resource)
    }
}
```

### 2. Multi-Provider Strategy

```rust
// volition-agent-core/src/strategies/plan_execute.rs
pub struct PlanExecuteStrategy {
    planning_provider: String,
    execution_provider: String,
}

impl PlanExecuteStrategy {
    pub fn new(planning_provider: String, execution_provider: String) -> Self {
        Self {
            planning_provider,
            execution_provider,
        }
    }
}

impl Strategy for PlanExecuteStrategy {
    fn name(&self) -> &'static str {
        "PlanExecute"
    }
    
    fn get_current_provider(&self, phase: StrategyPhase) -> String {
        match phase {
            StrategyPhase::Planning => self.planning_provider.clone(),
            StrategyPhase::Execution => self.execution_provider.clone(),
            _ => self.planning_provider.clone(), // Default
        }
    }
    
    // Implementation of plan/execute logic with provider switching
    async fn run(&mut self, agent: &mut Agent, state: &mut AgentState) -> Result<String, AgentError> {
        // Planning phase with planning provider
        agent.switch_provider(&self.planning_provider)?;
        let plan = self.generate_plan(agent, state).await?;
        
        // Execution phase with execution provider
        agent.switch_provider(&self.execution_provider)?;
        let result = self.execute_plan(agent, state, plan).await?;
        
        Ok(result)
    }
}
```

### 3. Agent with MCP Support

```rust
// volition-agent-core/src/agent.rs
pub struct Agent {
    providers: HashMap<String, Provider>,
    current_provider: String,
    mcp_clients: HashMap<String, Arc<McpClient>>,
    ui_handler: Arc<dyn UserInteraction>,
}

impl Agent {
    pub fn new(config: AgentConfig, ui_handler: Arc<dyn UserInteraction>) -> Result<Self> {
        // Initialize providers and MCP clients
        let mut providers = HashMap::new();
        let mut mcp_clients = HashMap::new();
        
        // Set up providers from config
        for (key, provider_config) in config.providers {
            providers.insert(key.clone(), Provider::new(provider_config)?);
        }
        
        // Set up MCP clients from config
        for (key, server_config) in config.mcp_servers {
            let client = McpClient::new(server_config.command, server_config.args)?;
            mcp_clients.insert(key, Arc::new(client));
        }
        
        Ok(Self {
            providers,
            current_provider: config.default_provider,
            mcp_clients,
            ui_handler,
        })
    }
    
    pub fn switch_provider(&mut self, provider_name: &str) -> Result<()> {
        if !self.providers.contains_key(provider_name) {
            return Err(anyhow!("Unknown provider: {}", provider_name));
        }
        self.current_provider = provider_name.to_string();
        Ok(())
    }
    
    pub async fn run(&mut self, strategy: Box<dyn Strategy>, initial_task: String) -> Result<String> {
        let mut state = AgentState::new(initial_task);
        
        // Connect to all MCP servers
        for (name, client) in &mut self.mcp_clients {
            client.connect().await?;
        }
        
        // Let the strategy drive the interaction
        strategy.run(self, &mut state).await
    }
    
    pub async fn get_completion(&self, messages: Vec<Message>) -> Result<ApiResponse> {
        let provider = self.providers.get(&self.current_provider)
            .ok_or_else(|| anyhow!("Current provider not found"))?;
        
        provider.get_completion(messages).await
    }
    
    pub async fn call_mcp_tool(&self, server: &str, tool: &str, args: Value) -> Result<Value> {
        let client = self.mcp_clients.get(server)
            .ok_or_else(|| anyhow!("MCP server not found: {}", server))?;
        
        client.call_tool(tool, args).await
    }
}
```

## Configuration Format

```toml
# New simplified Volition.toml format

system_prompt = """
You are Volition, an AI-powered software engineering assistant
specializing in Rust code analysis, debugging, refactoring, and
development...
"""

# Provider definitions (LLM APIs)
[providers]
  [providers.openai]
  type = "openai"
  model_name = "gpt-4o"
  api_key = "${OPENAI_API_KEY}"
  
  [providers.anthropic]
  type = "anthropic"
  model_name = "claude-3-5-sonnet"
  api_key = "${ANTHROPIC_API_KEY}"
  
  [providers.gemini]
  type = "gemini"
  model_name = "gemini-2-5-pro"
  api_key = "${GOOGLE_API_KEY}"

# MCP server definitions - all implemented in Rust
[mcp_servers]
  [mcp_servers.filesystem]
  command = "cargo"
  args = ["run", "--bin", "volition-filesystem-server"]
  
  [mcp_servers.shell]
  command = "cargo"
  args = ["run", "--bin", "volition-shell-server"]

# Simplified strategy provider configurations
[strategies]
  [strategies.plan_execute]
  planning_provider = "anthropic"  # Use Anthropic for planning
  execution_provider = "gemini"    # Use Gemini for execution
```

## Timeline

Total estimated time: **4 weeks** (reduced from 8 weeks)

| Phase | Duration | Primary Focus |
|-------|----------|---------------|
| 1     | 1 week   | MCP Foundation |
| 2     | 1 week   | Core Servers Implementation |
| 3     | 1 week   | Multi-Provider Agent |
| 4     | 1 week   | Testing and Polish |

## Next Steps

1. Add the `rmcp` crate to dependencies in Cargo.toml:
   ```toml
   [dependencies]
   rmcp = { git = "https://github.com/modelcontextprotocol/rust-sdk", branch = "main", features = ["server"] }
   ```

2. Create a prototype Rust MCP client using the official SDK
3. Implement a simple filesystem MCP server in Rust
4. Implement provider switching in the agent
5. Test with a simple plan/execute strategy
