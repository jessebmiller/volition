# Implementation Plan: Conversation Strategy for Volition

## 1. Overview

The goal is to create a `ConversationStrategy` that maintains context across multiple user interactions. This strategy will:
1. Run the `CompleteTaskStrategy` with full conversation history
2. Preserve all messages between interactions
3. Allow the user to provide follow-up questions or tasks
4. Present a seamless conversation experience

## 2. Design Changes

### 2.1 Strategy Interface Updates

Currently, the Strategy trait has methods that work with individual tasks. We need to modify this to handle full conversation histories:

```rust
// Current interface (simplified)
trait Strategy {
    fn name(&self) -> &'static str;
    fn initialize_interaction(&self, state: &mut AgentState) -> Result<NextStep, AgentError>;
    fn process_api_response(&self, state: &mut AgentState, response: ApiResponse) -> Result<NextStep, AgentError>;
    fn process_tool_results(&self, state: &mut AgentState, results: Vec<ToolResult>) -> Result<NextStep, AgentError>;
    fn process_delegation_result(&self, state: &mut AgentState, result: DelegationResult) -> Result<NextStep, AgentError>;
}
```

We don't need to change the Strategy trait itself, as the `AgentState` struct already contains the message history in its `messages` field. This design allows us to implement a conversation-aware strategy without modifying the trait interface.

### 2.2 New ConversationStrategy Implementation

We'll create a new strategy that wraps an inner strategy (typically `CompleteTaskStrategy`) and maintains conversation context across interactions:

```rust
pub struct ConversationStrategy {
    inner_strategy: Box<dyn Strategy + Send + Sync>,
    conversation_state: Option<AgentState>,
    end_current_task: bool,
}
```

### 2.3 Agent Interface Changes

The `Agent::run` method needs to be modified to return the updated conversation state along with the final message:

```rust
// Updated return type
pub async fn run(&mut self, working_dir: &Path) -> Result<(String, AgentState), AgentError>;
```

This will allow the main CLI loop to preserve the conversation state between runs.

## 3. Implementation Steps

### 3.1. Create the ConversationStrategy

First, we'll implement the new strategy in `volition-agent-core/src/strategies/conversation.rs`:

1. Implement the strategy that maintains conversation state
2. Add handling for transitioning between tasks
3. Add special handling for "exit" or "new conversation" commands

### 3.2. Update Agent Implementation

Modify the `Agent` implementation to:

1. Update the `run` method to return both the final message and agent state
2. Add a method to initialize an agent with an existing conversation state

### 3.3. Update the CLI Interface

Modify the CLI in `volition-cli/src/main.rs` to:

1. Use the `ConversationStrategy` instead of `CompleteTaskStrategy`
2. Maintain a single conversation state across multiple user inputs
3. Add commands to reset the conversation or exit

## 4. Detailed Implementation

### 4.1. Create ConversationStrategy Module

Create a new file: `volition-agent-core/src/strategies/conversation.rs`

```rust
use super::{DelegationInput, DelegationResult, NextStep, Strategy};
use crate::{AgentState, ApiResponse, ToolResult};
use crate::errors::AgentError;
use anyhow::anyhow;

pub struct ConversationStrategy {
    inner_strategy: Box<dyn Strategy + Send + Sync>,
    conversation_state: Option<AgentState>,
    end_current_task: bool,
}

impl ConversationStrategy {
    pub fn new(inner_strategy: Box<dyn Strategy + Send + Sync>) -> Self {
        Self {
            inner_strategy,
            conversation_state: None,
            end_current_task: false,
        }
    }

    pub fn with_state(
        inner_strategy: Box<dyn Strategy + Send + Sync>,
        existing_state: AgentState,
    ) -> Self {
        Self {
            inner_strategy,
            conversation_state: Some(existing_state),
            end_current_task: false,
        }
    }

    pub fn get_conversation_state(&self) -> Option<&AgentState> {
        self.conversation_state.as_ref()
    }
    
    pub fn get_conversation_state_mut(&mut self) -> Option<&mut AgentState> {
        self.conversation_state.as_mut()
    }
    
    pub fn take_conversation_state(&mut self) -> Option<AgentState> {
        self.conversation_state.take()
    }
}

impl Strategy for ConversationStrategy {
    fn name(&self) -> &'static str {
        "Conversation"
    }

    fn initialize_interaction(&self, state: &mut AgentState) -> Result<NextStep, AgentError> {
        // If we have existing conversation state, merge it with the new state
        if let Some(existing_state) = &self.conversation_state {
            // Create a new state that has all previous messages plus the new user message
            let user_message = state.messages.last().cloned().ok_or_else(|| {
                AgentError::Strategy("State contains no initial message".to_string())
            })?;
            
            // Use existing conversation messages but add the new user message
            state.messages = existing_state.messages.clone();
            state.add_message(user_message);
        }

        // Delegate to the inner strategy
        self.inner_strategy.initialize_interaction(state)
    }

    fn process_api_response(
        &self,
        state: &mut AgentState,
        response: ApiResponse,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_api_response(state, response)
    }

    fn process_tool_results(
        &self,
        state: &mut AgentState,
        results: Vec<ToolResult>,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_tool_results(state, results)
    }

    fn process_delegation_result(
        &self,
        state: &mut AgentState,
        result: DelegationResult,
    ) -> Result<NextStep, AgentError> {
        // Delegate to the inner strategy
        self.inner_strategy.process_delegation_result(state, result)
    }
}
```

### 4.2. Update Agent Implementation

Modify `volition-agent-core/src/lib.rs` to update the `Agent::run` method:

```rust
pub async fn run(&mut self, working_dir: &Path) -> Result<(String, AgentState), AgentError> {
    info!(working_dir = ?working_dir, strategy = self.strategy.name(), "Starting agent run.");

    let mut next_step = self
        .strategy
        .initialize_interaction(&mut self.state)
        .map_err(|e| {
            AgentError::Strategy(format!(
                "Initialization failed for {}: {}",
                self.strategy.name(),
                e
            ))
        })?;

    loop {
        trace!(next_step = ?next_step, "Processing next step.");
        match next_step {
            // ... existing match arms remain the same ...
            
            NextStep::Completed(final_message) => {
                info!("Strategy indicated completion.");
                trace!(message = %final_message, "Final message from strategy.");
                return Ok((final_message, self.state.clone()));
            }
        }
    }
}
```

Also, add a method to initialize an agent with existing conversation state:

```rust
pub fn with_conversation_state(
    config: RuntimeConfig,
    tool_provider: Arc<dyn ToolProvider>,
    ui_handler: Arc<UI>,
    strategy: Box<dyn Strategy + Send + Sync>,
    conversation_state: AgentState,
    new_user_message: String,
) -> Result<Self> {
    let http_client = Client::builder()
        .build()
        .context("Failed to build HTTP client for Agent")?;
    
    // Create a new state with just the user message
    let mut initial_state = AgentState::new(new_user_message);
    
    info!(
        strategy = strategy.name(),
        "Initializing Agent with strategy and existing conversation."
    );
    
    // The ConversationStrategy will handle merging the states
    Ok(Self {
        config,
        tool_provider,
        http_client,
        ui_handler,
        strategy,
        state: initial_state,
    })
}
```

### 4.3. Update mod.rs to Include the New Module

Update `volition-agent-core/src/strategies/mod.rs`:

```rust
pub mod complete_task;
pub mod conversation;  // Add this line

// Also export the new strategy
pub use conversation::ConversationStrategy;
```

### 4.4. Update CLI Main Loop

Modify `volition-cli/src/main.rs` to use the conversation strategy:

```rust
use volition_agent_core::{
    strategies::complete_task::CompleteTaskStrategy,
    strategies::conversation::ConversationStrategy,
    Agent, AgentState,
    // ... other imports
};

// ...

#[tokio::main]
async fn main() -> Result<()> {
    // ... existing setup code ...

    print_welcome_message();
    
    // Create a conversation state to track the entire conversation
    let mut conversation_state: Option<AgentState> = None;
    
    loop {
        println!("{}", "How can I help you?".cyan());
        print!("{} ", ">".green().bold());
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let trimmed_input = user_input.trim();

        if trimmed_input.is_empty() || trimmed_input.to_lowercase() == "exit" {
            break;
        }
        
        if trimmed_input.to_lowercase() == "new" {
            println!("{}", "Starting a new conversation...".cyan());
            conversation_state = None;
            continue;
        }

        // Create the agent with appropriate strategy
        let mut agent = if let Some(state) = conversation_state.take() {
            // Create a conversation strategy with existing state
            let inner_strategy = Box::new(CompleteTaskStrategy::new());
            let conversation_strategy = Box::new(
                ConversationStrategy::with_state(inner_strategy, state)
            );
            
            Agent::new(
                config.clone(),
                Arc::clone(&tool_provider),
                Arc::clone(&ui_handler),
                conversation_strategy,
                trimmed_input.to_string(),
            )?
        } else {
            // Create a new conversation strategy for first interaction
            let inner_strategy = Box::new(CompleteTaskStrategy::new());
            let conversation_strategy = Box::new(
                ConversationStrategy::new(inner_strategy)
            );
            
            Agent::new(
                config.clone(),
                Arc::clone(&tool_provider),
                Arc::clone(&ui_handler),
                conversation_strategy,
                trimmed_input.to_string(),
            )?
        };

        // Run the agent and get both response and updated state
        match agent.run(&project_root).await {
            Ok((final_message, updated_state)) => {
                info!("Agent session completed successfully for user input.");
                println!("{}", "--- Agent Response ---".bold());
                if let Err(e) = print_formatted(&final_message) {
                    error!(
                        "Failed to render final AI message markdown: {}. Printing raw.",
                        e
                    );
                    println!("{}", final_message);
                } else {
                    println!();
                }
                println!("----------------------");
                
                // Save the updated conversation state for next interaction
                conversation_state = Some(updated_state);
            }
            Err(e) => {
                println!(
                    "{}: {:?}\n",
                    "Agent run encountered an error".red(),
                    e
                );
            }
        }
    }

    println!("{}", "Thanks!".cyan());
    Ok(())
}
```

## 5. Optional Enhancements

### 5.1. Conversation Persistence

Add functionality to save and load conversation state to/from disk:

```rust
fn save_conversation_state(state: &AgentState, path: &Path) -> Result<()> {
    let serialized = serde_json::to_string_pretty(state)?;
    fs::write(path, serialized)?;
    Ok(())
}

fn load_conversation_state(path: &Path) -> Result<AgentState> {
    let serialized = fs::read_to_string(path)?;
    let state: AgentState = serde_json::from_str(&serialized)?;
    Ok(state)
}
```

### 5.2. System Message Context

Add a way to inject a system message at the start of each conversation to provide consistent context:

```rust
impl ConversationStrategy {
    pub fn with_system_message(
        inner_strategy: Box<dyn Strategy + Send + Sync>,
        system_message: String,
    ) -> Self {
        let mut initial_state = AgentState::new(String::new());
        initial_state.messages = vec![ChatMessage {
            role: "system".to_string(),
            content: Some(system_message),
            ..Default::default()
        }];
        
        Self {
            inner_strategy,
            conversation_state: Some(initial_state),
            end_current_task: false,
        }
    }
}
```

### 5.3. Conversation Management Commands

Add special commands to:
- View conversation history
- Clear specific messages
- Save or load conversations

## 6. Testing Plan

1. Unit tests for the ConversationStrategy
   - Test preservation of conversation state between tasks
   - Test correct handling of system messages

2. Integration tests for the Agent with ConversationStrategy
   - Test multi-turn conversations with tool usage
   - Test context awareness (references to previous messages)

3. End-to-end CLI tests
   - Test full conversation flow with multiple user inputs
   - Test conversation reset and exit commands

## 7. Implementation Timeline

1. Day 1: Implement the ConversationStrategy and update the Agent class
2. Day 2: Update the CLI interface and test basic conversation flow
3. Day 3: Add persistence, additional commands, and polish the experience
4. Day 4: Comprehensive testing and bug fixes

## 8. Potential Challenges

1. **Context Management**: The ConversationStrategy needs to carefully manage the merging of existing conversation state with new user input to avoid duplication or context loss.

2. **Memory Usage**: Long conversations will accumulate many messages. Consider adding a mechanism to truncate very old messages while preserving critical context.

3. **Tool Results**: Ensure that tool results from previous interactions are properly preserved in the conversation context for reference.

4. **Error Recovery**: If an error occurs mid-conversation, ensure the state can be recovered rather than losing the entire conversation.

## 9. Future Improvements

1. **Message Summarization**: For very long conversations, implement a way to summarize older messages to reduce token usage.

2. **Conversation Branching**: Allow the user to create forks/branches of a conversation to explore different directions.

3. **Conversation Templates**: Provide pre-configured conversation starters for common tasks.

4. **Export/Import**: Add functionality to export conversations to different formats or import them from external sources.

5. **Multi-Agent Conversations**: Enable conversations that involve multiple specialized agents working together.
