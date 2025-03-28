# Volition Architecture Overview

## 1. Purpose

Volition is an AI-powered command-line assistant specifically designed to aid Rust developers. It integrates deeply with the developer's environment, providing contextually aware assistance for tasks like code analysis, refactoring, understanding codebases, and executing development-related commands (like `git` or `cargo`). Unlike general-purpose assistants, Volition leverages specific "tools" to interact directly with the filesystem, code search utilities, version control, and the Rust build system, enabling it to perform concrete actions within a project.

## 2. High-Level Architecture

Volition follows a modular architecture centered around processing user requests, interacting with an AI model, executing tools based on the AI's instructions, and rendering the results back to the user.

The main components are:

1.  **CLI Interface**: Handles command-line argument parsing and user interaction.
2.  **Configuration**: Loads and provides access to settings.
3.  **Core Orchestration**: Manages the main application loop and coordinates other components.
4.  **AI Interaction**: Communicates with the configured large language model (LLM).
5.  **Tool Execution**: Implements and runs specific actions requested by the AI.
6.  **Output Rendering**: Formats and displays information to the terminal.

```mermaid
graph LR
    subgraph User Interaction
        CLI[CLI Interface / main.rs]
    end
    subgraph Core Logic
        Orchestration[Core Orchestration / main.rs]
        Config[Configuration / config.rs]
    end
    subgraph AI & Tools
        AI[AI Interaction / api.rs, models/]
        Tools[Tool Execution / tools/]
    end
    subgraph Output
        Rendering[Output Rendering / rendering.rs]
    end

    CLI --> Orchestration;
    Orchestration -- Loads --> Config;
    Orchestration -- Sends Request --> AI;
    AI -- Sends Response/Tool Calls --> Orchestration;
    Orchestration -- Executes --> Tools;
    Tools -- Returns Results --> Orchestration;
    Orchestration -- Sends Output --> Rendering;
    Rendering --> User;


3. Component Responsibilities & Interactions

• CLI Interface (src/main.rs):

  • Uses clap to parse command-line arguments (e.g., the user's query, verbosity flags).
  • Initiates the core application logic.
• Configuration (src/config.rs):

  • Defines the RuntimeConfig struct holding all configuration.
  • Loads settings from Volition.toml (using toml) and environment variables (using dotenvy).
  • Provides access to API keys, model endpoints, system prompts, and other operational parameters to the rest of the application.
• Core Orchestration (src/main.rs):

  • The heart of the application, likely within an async tokio runtime.
  • Initializes configuration via config.rs.
  • Manages the conversation history.
  • Takes the user's input query.
  • Constructs the request payload for the AI model, including the history and system prompt.
  • Calls the AI Interaction layer (api.rs) to get a response from the LLM.
  • Parses the AI's response. If the response contains requests to use tools:
* Identifies the requested tool(s) and arguments.
* Invokes the appropriate function(s) in the `Tool Execution` layer (`tools/`).
* Sends the tool execution results back to the AI for processing (potentially in another call to `api.rs`).
  • Once a final response is received from the AI, passes it to the Output Rendering layer.
• AI Interaction (src/api.rs, src/models/):

  • Contains functions to communicate with different AI providers (e.g., OpenAI, Gemini, Ollama).
  • Uses reqwest for making asynchronous HTTP requests to the AI model endpoints defined in the configuration.
  • Uses serde and structs/enums defined in src/models/ to serialize requests and deserialize responses (including chat messages, function/tool call
    specifications).
  • Handles API key authentication and potentially error handling/retries for API calls.
• Tool Execution (src/tools/):

  • Defines and implements the specific capabilities Volition can perform beyond just text generation.
  • Likely structured with a mod.rs and sub-modules for each tool (e.g., tools/shell.rs, tools/file_io.rs, tools/search.rs, tools/git.rs, tools/cargo.rs).
  • A dispatcher function (perhaps in tools/mod.rs) likely takes the tool call details from the AI response and routes execution to the correct tool
    implementation.
  • Implementations interact with the operating system or external processes (e.g., using duct for shell commands, standard library functions for file I/O,
    ignore or ripgrep via duct for search, potentially wrappers or direct commands for git and cargo).
  • Returns results (stdout, stderr, file contents, search results) back to the Core Orchestration layer, usually encapsulated in a Result.
• Output Rendering (src/rendering.rs):

  • Takes the final processed response content (usually Markdown) from the Core Orchestration layer.
  • Uses crates like termimad, pulldown-cmark, and syntect to parse Markdown, apply terminal-friendly styling, and perform syntax highlighting for code
    blocks.
  • Prints the formatted output to the standard output for the user to see.

4. Key Crates & Rationale (Briefly)

• tokio: Asynchronous runtime for handling concurrent operations like API calls and potential tool executions.
• clap: Standard for robust CLI argument parsing.
• reqwest: De facto standard for making HTTP requests (essential for API interaction).
• serde/serde_json: For serializing Rust structs into JSON for API requests and deserializing JSON responses.
• anyhow: Common choice for flexible error handling.
• tracing: For structured logging and diagnostics.
• termimad/pulldown-cmark/syntect: For rendering Markdown and syntax-highlighted code nicely in the terminal.
• duct: For safely and conveniently executing external shell commands (used by tools).
• ignore: For directory traversal that respects .gitignore rules (useful for file/search tools).

This structure allows for clear separation of concerns, making it easier to maintain, test, and extend the functionality (e.g., by adding new tools or
supporting different AI models).