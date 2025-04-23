---
title: Dogfooding an Agentic Coding Assistant 
date: 2025-03-25
tags: Rust, AI, VibeCoding, DevTooling
---

# Volition: An AI-Powered Software Engineering Assistant

## Project Overview

Volition is a command-line interface (CLI) tool designed to serve as an AI-powered software engineering assistant. The project leverages large language models to help developers analyze codebases, implement features, and refactor code through natural language interactions. Using a system of tools and strategies, Volition can execute shell commands, read and write files, search through code, and more—all while maintaining a conversation with the user.

### Key Features

- **Natural Language Code Interaction**: Communicate with your codebase through natural language queries
- **Code Analysis**: Analyze code patterns, architecture, and potential issues
- **Automated Refactoring**: Implement code changes based on natural language instructions
- **Intelligent Tool Usage**: Access to tools for shell commands, file operations, and code search
- **Multiple Strategy Support**: Includes both linear and simulated annealing strategies for solving complex problems

## Development Journey

### Phase 1: Initial Prototype with Claude.ai

The initial prototype was created through a conversation with Claude.ai, which generated a single Rust file containing the core functionality. This approach allowed for rapid prototyping and iteration.

Key aspects of this phase:
- Basic CLI structure implemented
- Core API integration with OpenAI established
- Simple conversation loop created
- Foundation for tool-based interactions laid

### Phase 2: Self-Improvement Capability

Once the basic functionality was in place, Volition gained the ability to modify and improve its own codebase. This represented a significant milestone in the project's development, as it enabled a more organic growth process where I could use the tool to:

- Restructure its own code into a more maintainable architecture
- Split functionality across multiple files and modules
- Implement new features based on user requests
- Optimize its own performance

### Phase 3: Advanced Problem-Solving Strategies

The project evolved to incorporate more sophisticated problem-solving approaches, particularly with the implementation of simulated annealing—an optimization algorithm that helps find better solutions through controlled randomization and progressive refinement.

Key features added during this phase:
- Git integration for tracking solution states
- Solution quality evaluation
- Temperature-based acceptance of new solutions
- Automatic tagging of optimal solutions

## Technical Implementation

### Architecture

Volition is built with a modular architecture that separates concerns into distinct components:

- **API Layer**: Handles communication with AI providers
- **Tools**: Implements specific capabilities (shell, file operations, code search)
- **Strategies**: Defines approaches for solving problems
- **Models**: Contains data structures for various components
- **Configuration**: Manages user settings and preferences
- **Main Application**: Orchestrates the components and handles user interaction

I continue to use Claude.ai to help describe the architectural patterns and refactoring plans. This structure was created by asking Claude.ai for a refactoring plan to help with maintainability. I copied the result into the repo then asked volition to implement that plan.

### Key Components

#### API Integration

The system supports multiple AI service providers through a unified interface:

```rust
pub async fn chat_with_api(
    client: &Client,
    config: &Config,
    messages: Vec<ResponseMessage>,
    tools: Vec<ToolDefinition>,
    temperature: Option<f64>,
) -> Result<ApiResponse> {
    // Use provided temperature or default from config
    let effective_temperature = temperature.unwrap_or_else(|| config.default_temperature.unwrap_or(0.2));

    match config.service.as_str() {
        "openai" => chat_with_openai(
            client,
            &config.openai_api_key,
            &config.model_name,
            messages,
            tools,
            effective_temperature,
        ).await,
        "ollama" => chat_with_ollama(client, &config.model_name, messages).await,
        _ => Err(anyhow!("Unsupported service: {}", config.service)),
    }
}
```

#### Tool System

The tool system provides a framework for extending Volition's capabilities:

```rust
pub async fn handle_tool_calls(
    _client: &Client,
    _api_key: &str,
    tool_calls: Vec<ToolCall>,
    messages: &mut Vec<ResponseMessage>,
) -> Result<()> {
    for (i, tool_call) in tool_calls.iter().enumerate() {
        info!(
            "Processing tool call #{}: id={}, name={}",
            i, tool_call.id, tool_call.function.name
        );

        match tool_call.function.name.as_str() {
            "shell" => {
                // Execute shell commands
            },
            "read_file" => {
                // Read file contents
            },
            // Additional tools...
        }
    }

    Ok(())
}
```

#### Problem-Solving Strategies

The system includes multiple strategies for approaching problems, from simple linear execution to advanced simulated annealing:

```rust
pub async fn linear_strategy(
    client: &Client,
    config: &Config,
    tools: Vec<Value>,
    end_tool: &str,
    mut messages: Vec<ResponseMessage>,
) -> Result<Vec<ResponseMessage>, anyhow::Error> {
    let mut conversation_active = true;

    while conversation_active {
        // Process messages and handle tool calls
        // until conversation completes
    }

    Ok(messages)
}
```

## User Experience

Volition provides a conversational interface that feels natural while offering powerful capabilities:

```
> volition "Analyze the src directory and list the main components"

Volition - AI Software Engineering Assistant
Ready to help you understand and improve your codebase.
Type 'exit' or press Enter on an empty line to quit at any time.

I'll analyze the src directory to identify the main components of your codebase.

Running: ls -la src/
... [output of command]

Let me read the main file to understand the project structure.

Reading file: src/main.rs
... [file content]

Based on my analysis, here are the main components of your codebase:

1. API Integration (src/api.rs): Handles communication with AI providers like OpenAI and Ollama
2. Configuration Management (src/config.rs): Manages user settings and API keys
3. Tool System (src/tools/): Provides capabilities for shell commands, file operations, and code searching
4. Strategy Implementation (src/strategies/): Implements different problem-solving approaches
5. Models (src/models/): Defines data structures used throughout the application
...

Would you like me to dive deeper into any particular component?

Enter a follow-up question or press Enter to exit:
> Tell me more about the simulated annealing strategy

Let me examine the simulated annealing implementation...
```

## Technical Challenges and Solutions

### Early challenge: Error Handling with External Tools

**Problem**: Volition would sometimes run shell commands that would error and stop the conversation.

**Solution**: I asked for tool use error handling and volition created a comprehensive error handling system using Anyhow for error propagation and detailed logging for debugging:

```rust
pub async fn run_shell_command(args: ShellArgs) -> Result<String> {
    // Error handling with context
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute Windows command")?
    } else {
        // Unix command execution with error context
    };

    // Additional error handling for command output
}
```
### Later challenge: Confusion around tool use

**Problem**: The Agent would often assume that calling the tools was something our code needed to do rather than something the AI agent would do.

**Solution**: Manually changed some poorly chosen and overloaded variable names, and asked Claude.ai to write some of the sections which I manually copied into the codebase.

## Lessons Learned

### 1. AI-Assisted Development

Working with AI to generate and improve code highlighted both the strengths and limitations of current AI systems:

- **Strengths**: Very fast implementation. Good models (GPT-4o) enerally use good practices.
- **Limitations**: The good models are expensive. They still need to be managed and guided by human experts.

### 2. Tool-Based AI Interaction

The tool-based interaction model proved highly effective, allowing for a clear separation of concerns while enabling powerful capabilities:

- Each tool has a specific purpose and well-defined interface
- New capabilities can be added by implementing additional tools
- Error handling can be centralized in the tool execution system

### 3. Strategy Pattern Benefits

The implementation of multiple problem-solving strategies demonstrated the value of this design pattern:

- Different approaches can be applied based on problem complexity
- Strategies can be swapped without changing the underlying system
- New strategies can be added without modifying existing code

### Separation of concerns for AI Agents

When concerns are separated into well named folders and files the AI can gather the context they need without extranious code.

- Reduces API cost by reducing the number of tokens in the context
- Reduces the chances the Agent will get confused by irrelevant code

### Rust constrained the AI Agent well

The strength of the Rust type system and borrow checker along with its verbose and helpful error messages gave the AI Agent lots of help when debugging.

## Future Development

### Planned Enhancements

1. **Additional Service Providers**: Support for more AI backends beyond OpenAI and Ollama
2. **Expanded Tool Set**: New tools for debugging, testing, and performance profiling
5. **Plugin System**: Allow for community-contributed tools and strategies
6. **Multi-Agent Teams**: Allow a team of agents each with a different model

### Architectural Improvements

1. **Caching System**: Reduce API calls by caching common operations
2. **Enhanced Testing**: Comprehensive test suite for all components
3. **Documentation Generator**: Automated documentation based on codebase analysis

## Conclusion

Volition shows what a single expert engineer can do quickly with the help of powerful tool using models like GPT-4o and Claude 3.7 Sonnet.

The project demonstrates how AI can be integrated into existing development workflows to enhance productivity without replacing the critical thinking and creativity that human developers bring to the table. As AI technology continues to advance, tools like Volition will likely become essential components in the modern developer\'s toolkit.
"""

# New case study text (ensure this is exactly the text agreed upon)
new_case_study = """
## Case Study: Debugging a Subtle Dependency Conflict with `pulldown-cmark`

**Date**: 2024-07-26

**Problem**: While integrating the `pulldown-cmark-to-cmark` crate to reconstruct Markdown strings from `pulldown_cmark::Event` streams for rendering with `termimad`, `cargo check` reported a persistent trait bound error:

```rust
error[E0277]: the trait bound `pulldown_cmark::Event<'_>: Borrow<pulldown_cmark::Event<'_>>` is not satisfied
    --> src/rendering.rs:151:5
     |
151  |     cmark(cloned_events.into_iter(), &mut md_string)
     |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Borrow<pulldown_cmark::Event<'_>>` is not implemented for `pulldown_cmark::Event<'_>`
     |
note: required by a bound in `cmark`
    --> /home/jesse/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/pulldown-cmark-to-cmark-21.0.0/src/lib.rs:1012:8
     | ...
1012 |     E: Borrow<Event<'a>>,
     |        ^^^^^^^^^^^^^^^^^ required by this bound in `cmark`
```

This error occurred within the `flush_markdown_buffer` function in `src/rendering.rs`, which buffered `Event`s before passing them to the `cmark` function. The `cmark` function expected an iterator whose items implemented `Borrow<Event<'a>>`.

**Troubleshooting Steps with AI Assistance (Volition/Gemini):**

1.  **Initial Analysis**: The AI assistant (Volition, powered by Gemini) correctly identified that the `cmark` function likely expected an iterator yielding references (`&Event`) rather than owned `Event`s, based on the `Borrow` trait bound.
2.  **Attempt 1 (`iter()`):** Modified the code to pass `events.iter()` (iterator of references) instead of `cloned_events.into_iter()` (iterator of owned events). This led to a similar error: `&pulldown_cmark::Event<'_>: Borrow<pulldown_cmark::Event<'_>>` not satisfied. This was confusing, as `&T` should implement `Borrow<T>`.
3.  **Attempt 2 (`into_iter()` with `mem::take`):** Hypothesizing that `cmark` might actually require owned events (based on its documentation examples), the code was changed to take ownership of the buffer (`mem::take`) and pass `events.into_iter()`. This brought back the original error (`Event<'_>: Borrow<Event<'_>>` not satisfied).
4.  **Attempt 3 (`into_owned()`):** Suspecting a lifetime mismatch related to cloned events potentially borrowing from the original input string, the AI suggested ensuring all events were fully owned using `Event::into_owned()`. This failed with a compiler error (`E0599`) because `into_owned` was not the correct method name/usage.
5.  **Attempt 4 (`to_owned()`):** Following the compiler's suggestion (`help: there is a method 'to_owned' with a similar name`), the code was changed to use `event.to_owned()`. This surprisingly brought back the very first error (`Event<'_>: Borrow<Event<'_>>` not satisfied).

**Diagnosis: Dependency Version Conflict**

The cyclical nature of the errors, despite trying both owned and borrowed approaches, strongly suggested a deeper issue. The AI proposed checking for conflicting versions of the `pulldown-cmark` crate itself.

*   Running `cargo tree -p pulldown-cmark` failed, indicating multiple versions were present (`0.11.3` and `0.13.0`).
*   Running `cargo tree -i pulldown-cmark@0.11.3` showed it was a direct dependency of the `volition` crate.
*   Running `cargo tree -i pulldown-cmark@0.13.0` showed it was required by `pulldown-cmark-to-cmark`.

This was the root cause: The `Event` type from `pulldown-cmark v0.11.3` (used in `volition`'s code) was fundamentally different from the `Event` type from `pulldown-cmark v0.13.0` (expected by the `cmark` function). The trait bound `Event<0.11.3>: Borrow<Event<0.13.0>>` could never be satisfied.

**Resolution:**

1.  The `Cargo.toml` file for the `volition` crate was updated to explicitly depend on `pulldown-cmark = "0.13.0"`, unifying the version used across the project.
2.  `cargo check` immediately passed after this change.
3.  The unnecessary `to_owned()` workaround in `src/rendering.rs` was reverted, and the more efficient `events.iter()` approach was successfully reinstated, confirming the type mismatch was the sole issue.
4.  The changes were committed.

**Conclusion:** This case study highlights how subtle dependency version conflicts in Rust can manifest as complex-seeming trait bound and lifetime errors. The iterative debugging process, guided by the AI assistant's analysis of error messages, exploration of different approaches, and eventual hypothesis of a version conflict, was crucial in pinpointing and resolving this non-obvious issue. Without the systematic exploration facilitated by the AI, diagnosing this problem would have been significantly more challenging.