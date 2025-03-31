# Strategies

Would be great to have a strategies abstraction

This would allow us to implment various ways of coding.

Examples:
- simple chat: user input -> agent run -> repeat
- TDD: user input -> prompt to write tests -> prompt to make tests pass -> repeat
- red green refactor: user input -> write tests -> pass tests -> refactor -> repeat
- plan, revise, execute: user input -> planning step -> revision -> plan good enough? -> execute or back to planning step -> repeat

future possibilities
- simulated annealing?!?


How can I help you?
> Yeah read that and look at the rest of the repo. Let me know if it's stale or if it's ready to be worked on
 143.085356442s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
 143.122411036s  INFO Starting agent run with 4 messages
 143.122425039s  INFO Starting agent run. num_initial_messages=4 working_dir="/home/jesse/work/volition"
 143.122432400s  INFO Agent iteration limit set to 20. limit=20
 143.122436574s  INFO Starting agent iteration 1/20. iteration=1 limit=20
 149.810624805s  INFO AI requested 1 tool call(s). count=1
 149.810660370s  INFO Executing tool via CliToolProvider tool_name="read_file" args={"path": String("project/in-progress/strategies.md")}
 149.810679419s  INFO Reading file (absolute): "/home/jesse/work/volition/project/in-progress/strategies.md"
 149.810701153s  INFO Read 523 bytes from file
 149.810708372s  INFO Tool 'read_file' executed successfully. tool_call_id= tool_name=read_file
 149.810734106s  INFO Starting agent iteration 2/20. iteration=2 limit=20
 151.685305578s  INFO AI requested 2 tool call(s). count=2
 151.685435042s  INFO Executing tool via CliToolProvider tool_name="list_directory" args={"path": String("."), "depth": Number(1)}
 151.690625851s  INFO Tool 'list_directory' executed successfully. tool_call_id= tool_name=list_directory
 151.690651128s  INFO Executing tool via CliToolProvider tool_name="git_command" args={"command": String("log"), "args": Array [String("-n"), String("5"), String("--oneline")]}
 151.690673178s  INFO Executing git command: git log -n 5 --oneline in "/home/jesse/work/volition"
 151.691949963s  INFO Tool 'git_command' executed successfully. tool_call_id= tool_name=git_command
 151.691985095s  INFO Starting agent iteration 3/20. iteration=3 limit=20
 154.677772931s  INFO Received final response from AI (no further tool calls requested).
 154.677928273s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Tool Execution Results:
- Tool: read_file, Status: Success
- Tool: list_directory, Status: Success
- Tool: git_command, Status: Success
Final AI Message:
Okay, I see the task description and the overall structure. We have:

• Cargo.toml and Cargo.lock at the root, suggesting a Cargo workspace.
• volition-agent-core/: Likely the core library for the agent logic.
• volition-cli/: A command-line interface using the core library.
• docs/, project/, README.md, etc.: Standard project files.
• The most recent commits are related to logging, merging a library extraction PR, and moving this 'strategies' task.

The task seems relevant and up-to-date, given the recent activity and the project structure. Extracting the core logic into volition-agent-core likely makes
implementing a strategy pattern easier now.

It looks ready to be worked on. How would you like to proceed with designing this "strategies abstraction"? We could start by looking at the existing
interaction loop in volition-cli or volition-agent-core to see where this abstraction would fit best.

-----------------------
 154.679681948s  INFO Removed recovery state file after successful run.
How can I help you?
> Yeah please put together a plan for the implementation. Let me know what questions you have about how the strategies should work
 215.548916229s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
 215.587644852s  INFO Starting agent run with 6 messages
 215.587658029s  INFO Starting agent run. num_initial_messages=6 working_dir="/home/jesse/work/volition"
 215.587665800s  INFO Agent iteration limit set to 20. limit=20
 215.587669801s  INFO Starting agent iteration 1/20. iteration=1 limit=20
 240.298792246s  INFO Received final response from AI (no further tool calls requested).
 240.298894971s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Final AI Message:
Okay, let's formulate a plan to implement the strategies abstraction.

Goal: Refactor the agent's core decision-making logic to use a Strategy trait, making it pluggable. We'll start by defining the trait and moving the
existing logic into a DefaultStrategy.

Plan:

1. Define the Strategy Trait:

   • Identify the inputs required for the agent to decide its next action (e.g., conversation history, available tools, current state).
   • Identify the possible outputs (e.g., a specific tool call, a request for user input, a final response).
   • Create a new module, likely volition-agent-core/src/strategies/mod.rs.
   • Define a public trait Strategy within this module with a core method like
     determine_next_action(&self, state: &CurrentStateType) -> Result<NextActionType, ErrorType>. We'll need to determine the exact types for
     CurrentStateType, NextActionType, and ErrorType by examining the existing code.
   • Tooling: read_file, write_file, find_rust_definition.
1. Implement DefaultStrategy:

   • Locate the existing code block where the agent currently decides what to do next (likely in volition-agent-core).
   • Create a struct DefaultStrategy in the strategies module.
   • Implement the Strategy trait for DefaultStrategy, moving the identified decision-making logic into the determine_next_action method.
   • Tooling: search_text, read_file, write_file.
1. Refactor Agent Core Logic:

   • Modify the main agent struct or function in volition-agent-core to hold an instance of a type implementing Strategy (e.g., Box<dyn Strategy>).
   • Replace the old hardcoded decision logic block with a call to the determine_next_action method on the stored strategy instance.
   • Tooling: read_file, write_file, cargo_command check.
1. Integration and Initialization:

   • Update the code where the agent is initialized (likely in volition-cli/src/main.rs or an agent constructor in the core library) to create an instance
     of DefaultStrategy and pass it to the agent.
   • Tooling: read_file, write_file, cargo_command check.
1. Verification:

   • Use cargo check and cargo clippy throughout the process to catch errors early.
   • Run cargo test to ensure existing functionality isn't broken.
   • Manually run the CLI (cargo run in volition-cli) to confirm the behavior is unchanged.
   • Tooling: cargo_command.
1. Commit Changes:

   • Commit the refactoring with a clear message (e.g., "refactor: Introduce Strategy pattern for agent decision logic").
   • Tooling: git_command add, git_command commit.

Questions to Clarify:

1. Decision Inputs: What specific information does the agent currently use to decide its next step? Is it just the last user message, the full conversation
history, available tools, or something else? We'll need to inspect the code, but your input is valuable.
1. Decision Outputs: What are the possible outcomes of the decision process? A specific tool to call with arguments? A request for more user input? A final
text response? An exit signal?
1. State Management: How is the agent's state (like conversation history) currently stored and passed around? Is there a central AppState struct?
1. Async Operations: Is the current decision-making logic async? If so, the Strategy trait's method will also need to be async.

Once we have a better understanding of these points (which we can get by examining the code), we can refine the signature of the Strategy trait and proceed
with the implementation.

How does this plan sound? Shall we start by examining the core agent logic in volition-agent-core to answer some of these questions and define the trait
signature?

-----------------------
 240.303532675s  INFO Removed recovery state file after successful run.
How can I help you?
> 1. I think it has the full chat message history which includes user messages, available tools, tool calls and results^[[D^[[D^[[D^[[D^[[D^[[D^[[D^[[D^[[D^oh assistant messages too. 2. looking at the examples in the todo file, the plan-revise-execute strategy might use two agents potentially with two different LLMs and endpoints. an engineer, a planner, and an evaluator, for instance. 3. no appstate struct, a backup of the conversation history is saved to a file . this could be improved. 4. its not currently async, though I think it could be valuable to be able to run multiple parts of a strategy in parallel. Definitely look around the repo to coroborate all this
 807.889055891s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
 807.921675168s  INFO Starting agent run with 8 messages
 807.921686785s  INFO Starting agent run. num_initial_messages=8 working_dir="/home/jesse/work/volition"
 807.921694869s  INFO Agent iteration limit set to 20. limit=20
 807.921698820s  INFO Starting agent iteration 1/20. iteration=1 limit=20
 829.330261956s  INFO Received final response from AI (no further tool calls requested).
 829.330388450s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Final AI Message:
Thanks for the clarifications! That gives us a much better picture, especially regarding the potential complexity of strategies and the desire for async
operations.

Okay, let's refine the plan incorporating this information:

Refined Plan:

1. Code Exploration (Core Logic & Types):

   • Dive into volition-agent-core/src to pinpoint the exact function(s) handling the main interaction loop or decision-making process.
   • Identify the concrete types used for:
 * Conversation messages (user, assistant, tool calls, tool results).
 * Tool definitions/schemas.
 * The representation of the "next action" chosen by the agent.
   • Check how LLM clients or other external resources are accessed.
   • Tooling: list_directory, search_text, read_file.
1. Define Strategy Trait (Async):

   • Create volition-agent-core/src/strategies/mod.rs.
   • Define pub trait Strategy: Send + Sync { ... }. We add Send + Sync bounds anticipating async usage.
   • Define the core async method. Based on your input, a likely signature is:

async fn determine_next_action(
    &self,
    // Context needed for the decision
    history: &[MessageType], // Use actual Message type
    available_tools: &[ToolDefinitionType], // Use actual Tool type
    // Resources needed to make the decision (e.g., LLM client)
    llm_client: &dyn LlmClientType, // Placeholder for LLM client trait/type
    // Potentially other resources...
) -> Result<NextActionType, ErrorType>; // Use actual Action and Error types


We need to figure out the exact types (MessageType, ToolDefinitionType, LlmClientType, NextActionType, ErrorType) from the code exploration step. The
strategy might need access to resources like an LLM client, so we should plan to pass those in.
Tooling: write_file.

Define NextAction Enum:

• Based on the code exploration, define a pub enum NextActionType (likely in the strategies module or a shared types module) representing the possible
  outcomes: e.g., CallTool(ToolCallData), SendResponse(String), RequestInput, Exit, Delegate(Box<dyn Strategy + Send + Sync>) (to support multi-agent
  strategies later).
• Tooling: write_file.

Implement DefaultStrategy:

• Create a struct DefaultStrategy (e.g., in volition-agent-core/src/strategies/default.rs).
• Implement impl Strategy for DefaultStrategy { ... }.
• Move the existing decision logic into the determine_next_action method. This might involve making parts of the existing logic async.
• The DefaultStrategy might need its own configuration or resources (like an LLM client handle) passed in during its creation.
• Tooling: read_file, write_file.

Refactor Agent Core Logic (Async):

• Modify the main agent struct in volition-agent-core to hold the strategy: strategy: Box<dyn Strategy + Send + Sync>.
• Make the main interaction loop/function async.
• Replace the old decision logic block with a call like: let action = self.strategy.determine_next_action(&history, &tools, &self.llm_client).await?;.
  Ensure necessary context and resources are passed.
• Tooling: read_file, write_file.

Integrate Async Runtime:

• Ensure volition-cli uses an async runtime (like tokio). Check volition-cli/Cargo.toml for dependencies and volition-cli/src/main.rs for the entry point
  (e.g., #[tokio::main]). Add tokio as a dependency if needed.
• Update the agent initialization in volition-cli (or wherever it happens) to create Box::new(DefaultStrategy::new(...)) and pass it to the agent
  constructor. Ensure the main function running the agent is async and awaited.
• Tooling: read_file, write_file, cargo_command add (potentially).

Verification:

• Use cargo check --workspace, cargo clippy --workspace frequently.
• Run cargo test --workspace.
• Manually run the CLI (cargo run --manifest-path volition-cli/Cargo.toml) to confirm behavior.
• Tooling: cargo_command.

Commit Changes:

• Commit the refactoring: git add ., git commit -m "refactor: Introduce async Strategy pattern for agent decision logic".
• Tooling: git_command.

This revised plan accounts for the async requirement and the potential complexity of strategies needing access to resources like LLM clients.

Shall we start with Step 1: Code Exploration? I can begin by listing the files in volition-agent-core/src and then searching for keywords related to the
interaction loop, message handling, or tool usage.

-----------------------
 829.365977757s  INFO Removed recovery state file after successful run.
How can I help you?
> Yeah please explore the code to get a good idea of what it's doing now and what it would take. then we can come back and refine the plan
1065.927373578s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
1065.962918367s  INFO Starting agent run with 10 messages
1065.962930520s  INFO Starting agent run. num_initial_messages=10 working_dir="/home/jesse/work/volition"
1065.962937860s  INFO Agent iteration limit set to 20. limit=20
1065.962941746s  INFO Starting agent iteration 1/20. iteration=1 limit=20
1071.612399261s  INFO AI requested 1 tool call(s). count=1
1071.612540046s  INFO Executing tool via CliToolProvider tool_name="list_directory" args={"path": String("volition-agent-core/src"), "depth": Number(2)}
1071.617059425s  INFO Tool 'list_directory' executed successfully. tool_call_id= tool_name=list_directory
1071.617199354s  INFO Starting agent iteration 2/20. iteration=2 limit=20
1224.044178232s  INFO AI requested 1 tool call(s). count=1
1224.044313485s  INFO Executing tool via CliToolProvider tool_name="search_text" args={"path": String("volition-agent-core/src"), "pattern": String("loop|process|message|tool|action|request"), "file_glob": String("*.rs")}
1224.047333413s  INFO Searching for pattern: 'loop|process|message|tool|action|request' in path: 'volition-agent-core/src' (glob: '*.rs', context: 1, ignore_case: true) -> max 50 lines
1224.057791267s  INFO Tool 'search_text' executed successfully. tool_call_id= tool_name=search_text
1224.057845331s  INFO Starting agent iteration 3/20. iteration=3 limit=20
1226.048776392s  INFO AI requested 1 tool call(s). count=1
1226.048838176s  INFO Executing tool via CliToolProvider tool_name="read_file" args={"path": String("volition-agent-core/src/lib.rs")}
1226.048872882s  INFO Reading file (absolute): "/home/jesse/work/volition/volition-agent-core/src/lib.rs"
1226.048950529s  INFO Read 16166 bytes from file
1226.048962643s  INFO Tool 'read_file' executed successfully. tool_call_id= tool_name=read_file
1226.049020227s  INFO Starting agent iteration 4/20. iteration=4 limit=20
1244.748709173s  INFO Received final response from AI (no further tool calls requested).
1244.748756721s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Tool Execution Results:
- Tool: list_directory, Status: Success
- Tool: search_text, Status: Success
- Tool: read_file, Status: Success
Final AI Message:
Okay, volition-agent-core/src/lib.rs contains the core logic!

Here's a summary of the findings:

• Main Structure: The Agent<UI: UserInteraction> struct holds the configuration, a ToolProvider, an HTTP client, and a UserInteraction handler.
• Core Loop: The Agent::run method implements the main interaction loop. It takes the message history and working directory.
• Loop Logic:
  1. Checks iteration limit; asks user via UserInteraction trait if limit is reached.
  1. Gets available tools using the ToolProvider trait.
  1. Calls api::get_chat_completion (which seems to be the LLM call) with the current message history and tools.
  1. Receives an ApiResponse containing choices, each with a ChatMessage.
  1. Appends the assistant's ChatMessage to the history.
  1. Decision Point: Checks if the assistant message contains tool_calls.
 * **If YES (Tool Calls):**
   * Iterates through the requested `tool_calls`.
   * Parses arguments.
   * Calls `ToolProvider::execute_tool` for each.
   * Formats the tool output (or error) as a `ChatMessage` with `role: "tool"`.
   * Collects `ToolExecutionResult` for the final output.
   * Appends the tool output messages to the history.
   * `continue`s the loop.
 * **If NO (Final Response):**
   * Extracts the final content from the assistant's message.
   * Creates the `AgentOutput` struct containing collected tool results and the final description.
   * Returns `Ok(AgentOutput)`.
• Key Types:
  • ChatMessage (from models::chat): Represents messages (user, assistant, tool). Seems central.
  • ToolDefinition, ToolCall (from models::tools): Define tools and their invocation requests.
  • ToolProvider (trait): Abstract way to get tool definitions and execute them.
  • UserInteraction (trait): Abstract way to handle user prompts (like the iteration limit).
  • ApiResponse (from models::chat): The structure returned by the LLM API.
  • AgentOutput: The final result of Agent::run.
• Async: The Agent::run, ToolProvider::execute_tool, UserInteraction::ask, and api::get_chat_completion methods are already async. This is great!

Refining the Plan:

The current structure is quite close to what we want. The "decision" is implicitly made by the LLM (whether it returns tool calls or a final message). The
Agent::run loop orchestrates this process.

Our Strategy trait should encapsulate the logic within the loop, specifically the part that takes the current state (history, tools) and decides what to do
next (call LLM, process tool calls, decide it's finished).

Revised Strategy Trait Idea:

Instead of replacing the entire loop, the Strategy could manage the progression within the loop.

// In volition-agent-core/src/strategies/mod.rs

use crate::{ChatMessage, ToolDefinition, /* other needed types */};
use crate::api::LlmApiClient; // Assuming an LLM client abstraction exists or can be made
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

// Possible states or actions the strategy can dictate
pub enum AgentDirective {
    // Call the LLM with the current history and tools
    QueryLlm(Vec<ChatMessage>, Vec<ToolDefinition>),
    // Execute these specific tool calls returned by the LLM
    ExecuteTools(Vec<ToolCall>), // Use actual ToolCall type
    // Provide the final response
    SendFinalResponse(Option<String>), // Content of the final assistant message
    // Request user input (e.g., for iteration limit)
    RequestUserInput(String, Vec<String>), // Prompt and options
    // Indicate an error occurred
    ReportError(anyhow::Error),
}

#[async_trait]
pub trait Strategy: Send + Sync {
    /// Determines the next step for the agent based on the current state.
    async fn determine_next_directive(
        &self,
        // Current state
        messages: &[ChatMessage],
        available_tools: &[ToolDefinition],
        iteration: usize,
        max_iterations: usize,
        // Potentially results from the previous step (e.g., LLM response, Tool results)
        last_llm_response: Option<&ApiResponse>, // Use actual ApiResponse type
        last_tool_results: Option<&[ToolExecutionResult]>, // Use actual ToolExecutionResult type
        // Resources
        // llm_client: Arc<dyn LlmApiClient>, // Pass necessary clients/resources
        // tool_provider: Arc<dyn ToolProvider>, // Maybe needed?
        // ui_handler: Arc<dyn UserInteraction>, // Maybe needed?
    ) -> Result<AgentDirective>;

    // Optional: A way to initialize or reset strategy-specific state if needed
    // async fn initialize_run(&mut self) -> Result<()> { Ok(()) }
}

Revised Agent::run Flow:

1. Initialize loop variables (iteration, collected results).
1. Initialize the strategy state (strategy.initialize_run().await?).
1. Loop:
   • Call strategy.determine_next_directive(...) providing current history, tools, iteration count, last response/results, etc.
   • match the returned AgentDirective:
 * `QueryLlm(msgs, tools)`: Call `api::get_chat_completion(msgs, tools)`. Store the response. Append assistant message to history. Go back to step 3,       
passing the response.                                                                                                                                       
 * `ExecuteTools(tool_calls)`: Iterate through calls, use `tool_provider.execute_tool`. Collect results and tool output messages. Append tool output        
messages to history. Go back to step 3, passing the tool results.                                                                                           
 * `SendFinalResponse(content)`: Construct `AgentOutput`. Return `Ok(AgentOutput)`.                                                                         
 * `RequestUserInput(prompt, opts)`: Call `ui_handler.ask`. Handle response (e.g., update max iterations or error out). Go back to step 3.                  
 * `ReportError(e)`: Return `Err(e)`.                                                                                                                       

Benefits of this Approach:

• Keeps the Agent::run structure relatively stable, focusing on orchestration.
• The Strategy trait encapsulates the decision logic about what to do next (call LLM, run tools, ask user, finish).
• Allows different strategies to implement complex flows (like the plan-revise-execute idea) by controlling the sequence of directives.
• Clearly separates orchestration (Agent::run) from strategy (impl Strategy).

Next Steps:

1. Confirm Key Types: We need the exact definitions for ChatMessage, ToolDefinition, ToolCall, ApiResponse (likely from models/chat.rs, models/tools.rs,
api.rs). Let's read those files.
1. Refine AgentDirective: Ensure the enum covers all necessary steps/transitions currently in Agent::run.
1. Refine Strategy Trait Signature: Finalize the inputs needed by determine_next_directive. Does it need direct access to the ToolProvider or
UserInteraction handler, or can the orchestrator handle that based on the directive? Passing less is generally better.
1. Implement DefaultStrategy: Port the logic from the current Agent::run loop into the DefaultStrategy::determine_next_directive method.

Does this revised approach seem sound? Shall we proceed by reading the model files (models/chat.rs, models/tools.rs) and api.rs to finalize the types and
trait signature?

-----------------------
1244.755539482s  INFO Removed recovery state file after successful run.
How can I help you?
> Could you walk me through how teh plan-revise-execute strategy would be implemented with this plan?
1706.514359460s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
1706.553291111s  INFO Starting agent run with 12 messages
1706.553304519s  INFO Starting agent run. num_initial_messages=12 working_dir="/home/jesse/work/volition"
1706.553311844s  INFO Agent iteration limit set to 20. limit=20
1706.553315712s  INFO Starting agent iteration 1/20. iteration=1 limit=20
1749.983122231s  INFO Received final response from AI (no further tool calls requested).
1749.983155199s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Final AI Message:
Okay, let's walk through how a hypothetical PlanReviseExecuteStrategy could be implemented using the proposed Strategy trait and AgentDirective enum.

The key idea is that the PlanReviseExecuteStrategy struct would need to maintain its own internal state to track which phase it's in (Planning, Executing,
Revising).

// Hypothetical internal state for the strategy
enum PlanReviseExecutePhase {
    NeedsPlan,
    ExecutingPlan { plan: String, step: usize }, // Store the plan and current step
    Revising { plan: String, execution_results: Vec<String> }, // Store results for revision
    Finished,
}

// The strategy struct
pub struct PlanReviseExecuteStrategy {
    // Internal state for the current run
    current_phase: PlanReviseExecutePhase,
    // Could hold configuration, e.g., planner prompt, executor prompt
    planner_prompt_template: String,
    executor_prompt_template: String,
    // Potentially references to different LLM configurations if needed
}

impl PlanReviseExecuteStrategy {
    pub fn new(/* config */) -> Self {
        Self {
            current_phase: PlanReviseExecutePhase::NeedsPlan,
            // ... initialize templates etc.
            planner_prompt_template: "Create a step-by-step plan to achieve: {goal}".to_string(),
            executor_prompt_template: "Execute step {step} of the plan: {plan}\nGiven context:\n{history}".to_string(),
        }
    }

    // Helper to reset state for a new run
    fn reset_state(&mut self) {
        self.current_phase = PlanReviseExecutePhase::NeedsPlan;
    }
}

#[async_trait]
impl Strategy for PlanReviseExecuteStrategy {
    // Optional: Reset internal state at the start of Agent::run
    // async fn initialize_run(&mut self) -> Result<()> {
    //     self.reset_state();
    //     Ok(())
    // }

    async fn determine_next_directive(
        &mut self,
        messages: &[ChatMessage],
        available_tools: &[ToolDefinition],
        iteration: usize,
        max_iterations: usize,
        last_llm_response: Option<&ApiResponse>,
        last_tool_results: Option<&[ToolExecutionResult]>,
        // llm_client: Arc<dyn LlmApiClient>, // Assuming access if needed directly
    ) -> Result<AgentDirective> {

        // --- Check iteration limits first ---
        if iteration >= max_iterations {
            // Could ask user or just report error/finish
             return Ok(AgentDirective::ReportError(anyhow::anyhow!("Max iterations reached")));
             // Or potentially: Ok(AgentDirective::RequestUserInput("Max iterations reached. Continue?".to_string(), vec!["Yes".to_string(), "No".to_string()]))
        }

        // --- Main State Machine Logic ---
        match &self.current_phase {
            PlanReviseExecutePhase::NeedsPlan => {
                println!("Strategy Phase: NeedsPlan");
                // 1. Construct the planning prompt using the history/initial request
                let goal = messages.first().map(|m| m.content.clone().unwrap_or_default()).unwrap_or_default(); // Simplified goal extraction
                let planner_prompt = self.planner_prompt_template.replace("{goal}", &goal);

                // 2. Create messages for the planner LLM call
                let mut planner_messages = vec![
                    ChatMessage::system("You are a planning assistant."),
                    ChatMessage::user(planner_prompt),
                ];
                 // Optionally add more history context if needed for planning

                // 3. Tell the orchestrator to call the LLM
                // Transition state internally *before* returning directive
                // self.current_phase = PlanReviseExecutePhase::ExecutingPlan { plan: String::new(), step: 0 }; // Placeholder, plan comes in next step
                Ok(AgentDirective::QueryLlm(planner_messages, available_tools.to_vec()))
            }

            // This state might be entered *after* the orchestrator gets the LLM response from the NeedsPlan step
            PlanReviseExecutePhase::ExecutingPlan { plan, step } => {
                println!("Strategy Phase: ExecutingPlan (Step {})", step);

                // --- Update state based on *previous* step's result ---
                let mut current_plan = plan.clone();
                if plan.is_empty() {
                     // We just received the plan from the LLM in the previous step
                    if let Some(response) = last_llm_response {
                         // Extract plan from response.content (needs careful parsing)
                         current_plan = response.choices[0].message.content.clone().unwrap_or_default();
                         println!("Received Plan: {}", current_plan);
                         self.current_phase = PlanReviseExecutePhase::ExecutingPlan { plan: current_plan.clone(), step: 0 };
                    } else {
                         return Ok(AgentDirective::ReportError(anyhow::anyhow!("Expected LLM response with plan, but got none")));
                    }
                }
                // --- Now decide the action for the current step ---

                // 1. Parse the plan to figure out the action for the current step (*step*)
                //    (This could involve regex, another LLM call for parsing, etc.)
                let action_for_step = format!("Action for step {} based on plan: {}", step, current_plan); // Placeholder

                // 2. Decide: Is it a tool call or another LLM query (e.g., executor LLM)?
                if action_for_step.contains("call_tool_A") { // Example logic
                    let tool_call = ToolCall { /* ... details ... */ id: "call_123".to_string(), function: FunctionCall { name: "ToolA".to_string(), arguments: "{}".to_string() } }; // Placeholder
                    // No state transition here yet, wait for tool result
                    Ok(AgentDirective::ExecuteTools(vec![tool_call]))
                } else {
                    // Assume we need the "Executor" LLM
                    let executor_prompt = self.executor_prompt_template
                        .replace("{step}", &step.to_string())
                        .replace("{plan}", &current_plan)
                        .replace("{history}", &format!("{:?}", messages)); // Basic history format

                    let executor_messages = vec![
                        ChatMessage::system("You are an execution assistant."),
                        ChatMessage::user(executor_prompt),
                    ];
                    // No state transition yet, wait for LLM result
                    Ok(AgentDirective::QueryLlm(executor_messages, available_tools.to_vec()))
                }
            }

            PlanReviseExecutePhase::Revising { plan, execution_results } => {
                 println!("Strategy Phase: Revising");
                 // --- Update state based on *previous* step's result ---
                 let mut results = execution_results.clone();
                 if let Some(tool_res) = last_tool_results {
                    // Append new tool results
                    results.extend(tool_res.iter().map(|r| format!("{:?}", r))); // Simple format
                 }
                 if let Some(llm_res) = last_llm_response {
                     // Append new LLM results
                     results.push(llm_res.choices[0].message.content.clone().unwrap_or_default());
                 }
                 self.current_phase = PlanReviseExecutePhase::Revising { plan: plan.clone(), execution_results: results.clone() };


                 // --- Decide next step based on revision ---
                 // 1. Evaluate the results (results) against the plan (plan).
                 //    (This could be simple heuristics or another LLM call for evaluation).
                 let evaluation_passed = !results.is_empty(); // Simple placeholder logic

                 if evaluation_passed {
                     // 2a. If execution is satisfactory, finish.
                     println!("Evaluation passed. Finishing.");
                     self.current_phase = PlanReviseExecutePhase::Finished;
                     // Find the final message to send (maybe the last result?)
                     let final_content = results.last().cloned();
                     Ok(AgentDirective::SendFinalResponse(final_content))
                 } else {
                     // 2b. If not satisfactory, decide: go back to planning or execution?
                     println!("Evaluation failed. Going back to planning.");
                     // For simplicity, let's restart planning
                     self.current_phase = PlanReviseExecutePhase::NeedsPlan;
                     // We need to indicate to the orchestrator to loop again,
                     // perhaps by querying the planner again? Or maybe just loop implicitly?
                     // Let's query the planner again with context about the failure.
                     let goal = messages.first().map(|m| m.content.clone().unwrap_or_default()).unwrap_or_default();
                     let planner_prompt = format!("The previous plan failed. Results: {:?}. Create a revised plan for the goal: {}", results, goal);
                     let planner_messages = vec![
                         ChatMessage::system("You are a planning assistant."),
                         ChatMessage::user(planner_prompt),
                     ];
                     Ok(AgentDirective::QueryLlm(planner_messages, available_tools.to_vec()))
                 }
            }

            PlanReviseExecutePhase::Finished => {
                println!("Strategy Phase: Finished");
                // Should not be reached if SendFinalResponse was returned previously
                 Ok(AgentDirective::ReportError(anyhow::anyhow!("Reached Finished state unexpectedly")))
            }
        }

        // --- State Transition Logic ---
        // This part is tricky. The state often needs to transition *based on the result*
        // of the directive we just returned. For example, after returning QueryLlm for the
        // planner, we expect the *next* call to determine_next_directive to have the
        // planner's response in `last_llm_response`. The logic at the *start* of the
        // matching branches needs to handle consuming these results and transitioning state.

        // Example refinement for ExecutingPlan state transition:
        // Inside the `ExecutingPlan` match arm, *after* returning a directive:
        // if directive was ExecuteTools {
        //    // Next state depends on tool result - stay in ExecutingPlan or move to Revising?
        //    // Let's assume we always revise after execution.
        //    self.current_phase = PlanReviseExecutePhase::Revising { plan: current_plan, execution_results: vec![] }; // Will be populated next cycle
        // } else if directive was QueryLlm {
        //    // Next state depends on LLM result - stay in ExecutingPlan or move to Revising?
        //    // Let's assume we always revise after execution.
        //    self.current_phase = PlanReviseExecutePhase::Revising { plan: current_plan, execution_results: vec![] }; // Will be populated next cycle
        // }
        // **Correction**: This logic needs to happen at the START of the *next* call, using the `last_llm_response` or `last_tool_results`. The code above tries to put this logic within the `match` arms.
    }
}

How it Interacts with Agent::run:

1. Start: Agent::run calls determine_next_directive.
   • Strategy: Is in NeedsPlan. Returns AgentDirective::QueryLlm(planner_prompt, ...).
1. Orchestrator: Receives QueryLlm. Calls the LLM, gets the plan response. Appends plan request/response to history. Loops.
1. Next Iteration: Agent::run calls determine_next_directive again, passing the updated history and the last_llm_response (containing the plan).
   • Strategy: Sees last_llm_response is present. Uses its internal logic (maybe still in NeedsPlan or transitioned based on previous step). Let's say it
     transitions to ExecutingPlan { plan: extracted_plan, step: 0 }. It parses step 0, determines it needs ToolA. Returns
     AgentDirective::ExecuteTools(vec![tool_call_for_ToolA]).
1. Orchestrator: Receives ExecuteTools. Executes ToolA. Gets results. Appends tool request/result messages to history. Loops.
1. Next Iteration: Agent::run calls determine_next_directive again, passing updated history and last_tool_results.
   • Strategy: Sees last_tool_results. Uses its internal logic (maybe still in ExecutingPlan or transitions). Let's say it transitions to
     ExecutingPlan { plan: ..., step: 1 } or maybe Revising { plan: ..., results: ... }. It decides the next action based on the revision or the next step.
     Returns another AgentDirective (e.g., QueryLlm for executor, ExecuteTools for step 1, or SendFinalResponse if done).
1. Loop Continues: This cycle repeats. The strategy uses its internal state (current_phase) and the results from the previous step (last_llm_response,
last_tool_results) to decide which AgentDirective to return next.
1. End: Eventually, the strategy's internal logic decides the process is complete. It returns AgentDirective::SendFinalResponse(content).
1. Orchestrator: Receives SendFinalResponse. Constructs AgentOutput and returns Ok.

This shows how the separation works: Agent::run handles the mechanics of calling LLMs/tools and managing history, while the Strategy implementation contains
the complex state machine and logic dictating the sequence of those mechanics.

-----------------------
1750.009185836s  INFO Removed recovery state file after successful run.
How can I help you?
> I think you're confused about how the tools work. These are not tools we call ourselves, These are tools for the LLM to call we just execute them when they do and pass the results back. I could see us making a tool for the LLM to submit_plan in a structured way. This would make extracting it simple, and would make it clear when to move on to EvaluatingPlan. which would be another call to the LLM (get_chat_completion) with an evaluator prompt + plan and a tool to submit a score (say 1-10, or 0-1) in the same way. The strategy could then decide what to do based on the score. either revise again, or move on to executing. etc. The strategy can really only respond to tool calls from the LLM and it can either make algorithmic decisions based on the LLM tool calls or hand a decision to an llm with a prompt and some tools to direct the algorithm.
2279.116295656s  INFO Saved conversation state to "/home/jesse/work/volition/.conversation_state.json"
2279.146752880s  INFO Starting agent run with 14 messages
2279.146764156s  INFO Starting agent run. num_initial_messages=14 working_dir="/home/jesse/work/volition"
2279.146771388s  INFO Agent iteration limit set to 20. limit=20
2279.146775106s  INFO Starting agent iteration 1/20. iteration=1 limit=20
2327.527784408s  INFO Received final response from AI (no further tool calls requested).
2327.527892487s  INFO Agent run finished successfully.
--- Agent Run Summary ---
Final AI Message:
You are absolutely right! My apologies, I was mixing up the agent initiating tool calls with the agent executing tool calls requested by the LLM. Thank you
for the correction.

Your explanation clarifies the flow significantly: the Strategy primarily reacts to the LLM's output (text and tool calls) and decides how to proceed, often
by preparing the next set of messages for the LLM, potentially including results from executed tools.

Let's refine the plan based on this correct understanding.

Revised Plan (Take 3):

1. Code Exploration (Confirmation):

   • Quickly re-verify in volition-agent-core/src/lib.rs (Agent::run) that the flow is indeed: LLM Call -> Check Response for Tool Calls -> Execute Tools ->
     Send Results Back to LLM -> Repeat. (From our previous look, this seems correct).
   • Confirm the exact types: ChatMessage, ToolDefinition, ToolCall, ApiResponse, ToolExecutionResult.
   • Tooling: read_file (volition-agent-core/src/lib.rs, models/chat.rs, models/tools.rs).
1. Define Strategy Trait & StrategyDecision Enum:

   • Create volition-agent-core/src/strategies/mod.rs.
   • Define the StrategyDecision enum as discussed:

pub enum StrategyDecision {
    /// Provide the messages for the *next* LLM query.
    QueryLlm(Vec<ChatMessage>),
    /// Request execution of tool calls received from the *last* LLM response.
    /// The orchestrator handles execution and loops back with results.
    ExecuteTools(Vec<ToolCall>), // Contains the specific calls the LLM requested
    /// Interaction is complete. Provide final assistant message content.
    SendFinalResponse(Option<String>),
    /// Report an unrecoverable error.
    ReportError(anyhow::Error),
}


Define the Strategy trait:

use crate::models::chat::{ApiResponse, ChatMessage, ToolCall};
use crate::models::tools::{ToolDefinition, ToolExecutionResult};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Strategy: Send + Sync {
    /// Called at the very beginning of an agent run to get initial messages.
    /// Typically includes a system prompt and the initial user request.
    fn initialize_interaction(&self, user_prompt: &str, available_tools: &[ToolDefinition]) -> Vec<ChatMessage>;

    /// Process the latest LLM response and decide the next step.
    /// This is called *after* an LLM response is received.
    async fn process_llm_response(
        &mut self,
        history: &[ChatMessage], // History *including* the latest assistant response
        response: &ApiResponse, // The LLM response to process
        available_tools: &[ToolDefinition],
    ) -> Result<StrategyDecision>;

    /// Process the results of tool executions and decide the next step.
    /// This is called *after* tools (requested previously by the LLM) have been executed.
    async fn process_tool_results(
        &mut self,
        history: &[ChatMessage], // History *including* the tool result messages
        tool_results: &[ToolExecutionResult], // Results of the tools just executed
        available_tools: &[ToolDefinition],
    ) -> Result<StrategyDecision>;

    // Optional: Add methods for handling errors, iteration limits, etc. if needed
}

Self-Correction: Separating process_llm_response and process_tool_results makes the flow clearer than trying to handle both in one method using Options. The
orchestrator knows whether it just got an LLM response or tool results and can call the appropriate strategy method.
Tooling: write_file.

Implement DefaultStrategy:

• Create volition-agent-core/src/strategies/default.rs.
• Implement Strategy for DefaultStrategy.
• initialize_interaction: Creates the initial system prompt + user message.
• process_llm_response: Mimics the current logic: Check response.choices[0].message.tool_calls. If present, return StrategyDecision::ExecuteTools(calls). If
  not, return StrategyDecision::SendFinalResponse(response.choices[0].message.content).
• process_tool_results: Mimics the current logic: Format tool_results into ChatMessage::tool(...) messages. Append them to the history. Return
  StrategyDecision::QueryLlm(updated_history).
• Tooling: read_file, write_file.

Refactor Agent::run:

• Modify Agent struct to hold strategy: Box<dyn Strategy + Send + Sync>.
• Modify Agent::new (or constructor) to accept the strategy.
• Rewrite the Agent::run loop:
  • Get initial messages: let mut history = self.strategy.initialize_interaction(&initial_prompt, &tools);
  • Loop:
* Call LLM: `let response = self.llm_client.get_chat_completion(&history, &tools).await?;`                                                                  
* Append assistant message: `history.push(response.choices[0].message.clone());`                                                                            
* Process response: `let decision = self.strategy.process_llm_response(&history, &response, &tools).await?;`                                                
* Handle decision:                                                                                                                                          
  * `SendFinalResponse(content)`: Break loop, return `Ok(AgentOutput { final_content: content, ... })`.                                                     
  * `ReportError(e)`: Return `Err(e)`.                                                                                                                      
  * `ExecuteTools(calls)`:                                                                                                                                  
    * Execute tools: `let results = self.execute_requested_tools(calls).await?; // Helper needed`                                                           
    * Append tool result messages to `history`.                                                                                                             
    * Process results: `let decision2 = self.strategy.process_tool_results(&history, &results, &tools).await?;`                                             
    * Handle `decision2`:                                                                                                                                   
      * `QueryLlm(next_history)`: `history = next_history; continue;` (Loop back to LLM call)                                                               
      * `SendFinalResponse(content)`: Break loop, return `Ok(AgentOutput { ... })`.                                                                         
      * `ReportError(e)`: Return `Err(e)`.                                                                                                                  
      * `ExecuteTools(_)`: Likely an error/unexpected state from `process_tool_results`. Return `Err(...)`.                                                 
  * `QueryLlm(next_history)`: `history = next_history; continue;` (Should not happen directly after LLM response in default flow, but possible for complex  
strategies).                                                                                                                                                
• Need a helper function execute_requested_tools that takes Vec<ToolCall> and returns Result<Vec<ToolExecutionResult>>.
• Tooling: read_file, write_file, cargo_command check.

Integration:

• Update volition-cli/src/main.rs (or agent creation site) to instantiate DefaultStrategy and pass it to the Agent.
• Tooling: read_file, write_file, cargo_command check.

Verification:

• cargo check --workspace, cargo clippy --workspace.
• cargo test --workspace.
• Manual run via CLI.
• Tooling: cargo_command.

Commit Changes:

• git add ., git commit -m "refactor: Introduce Strategy pattern reacting to LLM/tool results".
• Tooling: git_command.

Plan-Revise-Execute Strategy Example (Take 3):

• initialize_interaction: Sets up initial prompt asking the LLM to create a plan and call submit_plan(plan_text). Sets internal state to NeedsPlan.
• process_llm_response:
  • If state is NeedsPlan: Expects submit_plan call. If found, extract plan, store it, change state to EvaluatingPlan. Return
    StrategyDecision::ExecuteTools(vec![submit_plan_call]) (even if execution is trivial, it keeps the flow consistent). If no submit_plan call, maybe
    reprompt or error.
  • If state is EvaluatingPlan: Expects submit_evaluation call. Extract score, change state based on score (e.g., ExecutingStep or RevisingPlan). Return
    StrategyDecision::ExecuteTools(vec![submit_evaluation_call]).
  • If state is ExecutingStep: Expects work-related tool calls (read_file, etc.) or maybe just text. If tool calls, return
    StrategyDecision::ExecuteTools(calls). If just text ("Step N complete"), update step counter, prepare prompt for next LLM call (e.g., asking for next
    step's tools or confirming plan completion) and return StrategyDecision::QueryLlm(next_prompt).
  • If state is RevisingPlan: Expects submit_plan call (with the revised plan). Extract plan, store it, change state to EvaluatingPlan. Return
    StrategyDecision::ExecuteTools(vec![submit_plan_call]).
• process_tool_results:
  • If state was NeedsPlan (after submit_plan executed): Prepare evaluation prompt (using the stored plan), provide submit_evaluation tool. Change state to
    EvaluatingPlan. Return StrategyDecision::QueryLlm(eval_prompt).
  • If state was EvaluatingPlan (after submit_evaluation executed): Check stored score. If high, prepare prompt for first execution step, provide work
    tools, change state to ExecutingStep(0). If low, prepare revision prompt, provide submit_plan tool, change state to RevisingPlan. Return
    StrategyDecision::QueryLlm(...).
  • If state was ExecutingStep (after work tool executed): Append results to history. Prepare prompt for LLM (
    "Tool results: {...}. What is the next tool call for step N?"). Return StrategyDecision::QueryLlm(results_prompt). (State remains ExecutingStep).
  • If state was RevisingPlan (after submit_plan executed): Prepare evaluation prompt for the revised plan, provide submit_evaluation tool. Change state to
    EvaluatingPlan. Return StrategyDecision::QueryLlm(eval_prompt).

This feels much more robust and correctly models the interaction flow.

Shall we proceed with Step 1: Confirming Types by reading the relevant model and API files?

-----------------------
2327.537757078s  INFO Removed recovery state file after successful run.
How can I help you?
>