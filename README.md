# Volition

<div align="center">

![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
[![Stars](https://img.shields.io/github/stars/jessebmiller/volition?style=social)](https://github.com/jessebmiller/volition/stargazers)

**Volition is a contextually-aware AI assistant designed for Rust development workflows, built with a modular architecture.**

[Getting Started](#getting-started) •
[Features](#key-features) •
[Installation](#installation) •
[Configuration](#configuration) •
[Usage](#usage) •
[Community](#community) •
[Contributing](#contributing)

</div>

---

## What is Volition?

Volition is an open-source CLI tool that brings the power of advanced AI assistants directly into your development workflow. Unlike many existing tools that either generate new code in isolation or offer general programming advice, Volition leverages a modular architecture to integrate deeply with your codebase and development tools, providing contextually-aware assistance.

It consists of:
*   **`volition-agent-core`**: A library handling agent logic, AI model interaction (via Providers), strategies, and communication with tool servers.
*   **Tool Servers**: Separate processes (e.g., for filesystem, git, shell commands) exposing capabilities via the Multi-Component Protocol (MCP).
*   **`volition-cli`**: The command-line interface that uses the core library to interact with the user and orchestrate tasks.

> 💡 **Built for Rust developers** - Volition understands Rust idioms and best practices.

```ignore
$ volition

Volition - AI Assistant
Type 'exit' or press Enter on an empty line to quit.
Type 'new' to start a fresh conversation.

How can I help you?
> Find references to AgentConfig across the codebase and explain how it's used
I found several references to `AgentConfig` in `volition-agent-core` and `volition-cli`:

1.  **Definition (`volition-agent-core/src/config.rs`):** Defines the main configuration structure, loading settings for providers, MCP servers, and strategies from `Volition.toml`.
2.  **Loading (`volition-cli/src/main.rs`):** Loaded at startup using `load_cli_config()` which reads and parses `Volition.toml`.
3.  **Agent Initialization (`volition-agent-core/src/agent.rs`):** An `AgentConfig` instance is required by `Agent::new` to configure providers and locate MCP servers.

It centralizes configuration for the agent's components. Let me know if you want details on a specific part!
----------------------

How can I help you?
>
```

## Key Features

• Modular Architecture: Core agent logic separated from tool implementations (filesystem, git, shell, search) running as distinct MCP servers.
• Multi-Provider Support: Interact with various AI models (Gemini, Ollama, OpenAI, and potentially others) through a unified Provider interface in volition-agent-core.
• Extensible Strategies: Implement different agent behaviors (e.g., CompleteTask, PlanExecute, more in the future) using the Strategy trait.
• Contextual Tool Use
  : The agent intelligently calls tools exposed by MCP servers based on the AI's requests (e.g., reading files, running git commands, searching code).
• Conversation Management: The CLI wrapper maintains conversation history across turns.
• Privacy Focused: Your code stays on your machine. Only necessary queries and tool arguments/results are exchanged (this will often include much of the code sent to the API you are using. If this is Ollama running locally it's fully local, though Ollama models are often not powerful enough if you are running on consumer hardware).

Installation

Prerequisites

• Rust: [Install Rust](https://www.rust-lang.org/tools/install) (required to build from source)
• ripgrep (rg): Required for the code search server (volition-search-server)
  • macOS: brew install ripgrep
  • Ubuntu/Debian: apt install ripgrep
  • Windows: scoop install ripgrep or choco install ripgrep
• git: Required for the git server (volition-git-server)
  • Install via your system's package manager (e.g., brew install git, apt install git).

Build from Source

# Clone the repository
git clone https://github.com/jessebmiller/volition.git
cd volition

# Build the entire workspace (core, servers, cli)
cargo build --release --workspace

# Optional: Add the main CLI binary to your PATH
# The servers will be run automatically by the agent core as needed.
cp target/release/volition ~/.local/bin/   # Adjust for your preferred bin directory (Unix)
# or
# copy target\\release\\volition.exe %USERPROFILE%\\bin\\   # Adjust for your preferred bin directory (Windows)

Configuration

Volition uses a Volition.toml file placed at the root of your project (it searches upwards from the current directory).

1. Create Volition.toml in your project root.
1. Configure API keys via environment variables specified in the config file.

Example Volition.toml:

# Default AI provider to use if not specified otherwise
default_provider = "gemini" # Must match a key under [providers]

# --- AI Model Providers ---
[providers]
  # Configuration for a Google Gemini provider instance
  [providers.gemini]
  provider_type = "gemini" # Type identifier ("gemini" or "ollama")
  # Environment variable containing the API key
  api_key_env_var = "GEMINI_API_KEY"
  # Model-specific configuration
  [providers.gemini.model_config]
    # Actual model name for the API call (e.g., "gemini-1.5-flash-latest")
    model_name = "gemini-1.5-flash-latest"
    # API endpoint URL
    endpoint = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent"
    # Optional parameters like temperature, max_tokens etc.
    # parameters = { temperature = 0.7, maxOutputTokens = 1024 }

  # Configuration for a local Ollama provider instance
  [providers.ollama_llama3]
  provider_type = "ollama"
  api_key_env_var = "" # Ollama typically doesn't require an API key
  [providers.ollama_llama3.model_config]
    model_name = "llama3" # Model name known to your Ollama instance
    endpoint = "http://localhost:11434/api/chat" # Default Ollama endpoint
    # parameters = { temperature = 0.5 }

# --- MCP Tool Servers ---
# Configure how the agent should run the tool servers.
[mcp_servers]
  [mcp_servers.filesystem]
  # Command to execute the filesystem server binary
  command = "volition-filesystem-server"
  # Arguments to pass (usually none needed)
  # args = []

  [mcp_servers.git]
  command = "volition-git-server"

  [mcp_servers.search]
  command = "volition-search-server"

  # Add other servers like 'shell' if implemented

# --- Strategy Configurations ---
# Define settings for different agent strategies
[strategies]
  [strategies.plan_execute]
  # Provider to use for the planning phase
  planning_provider = "gemini" # Must match a key under [providers]
  # Provider to use for the execution phase
  execution_provider = "gemini" # Must match a key under [providers]
  # Optional: Add other strategy-specific settings here
  # max_steps = 10

# You can define configs for other strategies here too
# [strategies.another_strategy]
# setting = "value"
# Default AI provider to use if not specified otherwise
default_provider = "gemini" # Must match a key under [providers]

# --- AI Model Providers ---
[providers]
  # Configuration for a Google Gemini provider instance
  [providers.gemini]
  provider_type = "gemini" # Type identifier ("gemini" or "ollama")
  # Environment variable containing the API key
  api_key_env_var = "GEMINI_API_KEY"
  # Model-specific configuration
  [providers.gemini.model_config]
    # Actual model name for the API call (e.g., "gemini-1.5-flash-latest")
    model_name = "gemini-1.5-flash-latest"
    # API endpoint URL
    endpoint = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent"
    # Optional parameters like temperature, max_tokens etc.
    # parameters = { temperature = 0.7, maxOutputTokens = 1024 }

  # Configuration for a local Ollama provider instance
  [providers.ollama_llama3]
  provider_type = "ollama"
  api_key_env_var = "" # Ollama typically doesn't require an API key
  [providers.ollama_llama3.model_config]
    model_name = "llama3" # Model name known to your Ollama instance
    endpoint = "http://localhost:11434/api/chat" # Default Ollama endpoint
    # parameters = { temperature = 0.5 }

# --- MCP Tool Servers ---
# Configure how the agent should run the tool servers.
[mcp_servers]
  [mcp_servers.filesystem]
  # Command to execute the filesystem server binary
  command = "volition-filesystem-server"
  # Arguments to pass (usually none needed)
  # args = []

  [mcp_servers.git]
  command = "volition-git-server"

  [mcp_servers.search]
  command = "volition-search-server"

  # Add other servers like 'shell' if implemented

# --- Strategy Configurations ---
# Define settings for different agent strategies
[strategies]
  [strategies.plan_execute]
  # Provider to use for the planning phase
  planning_provider = "gemini" # Must match a key under [providers]
  # Provider to use for the execution phase
  execution_provider = "gemini" # Must match a key under [providers]
  # Optional: Add other strategy-specific settings here
  # max_steps = 10

# You can define configs for other strategies here too
# [strategies.another_strategy]
# setting = "value"

Explanation:

• default_provider: Specifies which provider configuration under [providers] to use by default.
• [providers]: Defines different AI model services.
  • Each sub-section (e.g., [providers.gemini]) needs a provider_type ("gemini" or "ollama").
  • api_key_env_var: The name of the environment variable holding the API key (e.g., export GEMINI_API_KEY=your_key_here). Leave empty if no key is needed.
  • [providers.*.model_config]: Contains the model_name (specific model identifier for the API) and the API endpoint. parameters are optional model-specific
    settings.
• [mcp_servers]: Tells the agent how to launch the required tool servers. The command should be the name of the server binary (assuming it's in your PATH).
• [strategies]: Configures specific strategies. For plan_execute, you define which providers handle planning and execution.

Usage

Basic Usage

# Navigate to your project directory (where Volition.toml is or in a subdirectory)
cd /path/to/your/project

# Set required environment variables (if not already set in your shell profile)
export GEMINI_API_KEY=your_gemini_key_here

# Start an interactive session with an initial task
volition "Help me understand the architecture of this codebase"

# Increase verbosity for debugging (-v, -vv)
volition -v "Refactor the error handling in src/utils.rs"

Agent Operation

When you provide a task:

1. The volition-cli starts the Agent from volition-agent-core.
1. The Agent selects a Strategy (e.g., PlanExecute or CompleteTask, potentially wrapped by Conversation).
1. The Strategy interacts with the configured Provider (e.g., "gemini").
1. The Provider sends requests to the AI model API.
1. If the AI requests a tool call (e.g., "read file src/main.rs"), the Agent identifies the responsible MCP server (e.g., "filesystem").
1. The Agent ensures the required MCP server is running (launching it via the configured command if necessary) and establishes a connection.
1. The tool call is sent to the MCP server, executed, and the result is returned to the Agent.
1. The result is passed back to the AI model via the Provider.
1. This loop continues until the Strategy determines the task is complete.
1. The final response is displayed by volition-cli.

Philosophy

Volition is committed to remaining free and open-source forever. While some services may be built around it in the future (such as hosted vector databases
for improved context), the core tool will always be free to use, with users only paying for their own API costs.

We believe AI assistants should:

1. Augment workflows rather than replace developers
1. Respect privacy by keeping code on your machine
1. Integrate deeply with existing development tools
1. Remain transparent in how they work

Community

• GitHub Discussions: [Join the conversation](https://github.com/jessebmiller/volition/discussions)
• Discord: [Join our community](https://discord.gg/example) <!-- Replace with actual link -->
• Twitter: Follow [@VolitionAI](https://twitter.com/example) for updates <!-- Replace with actual link -->

Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for more details on how to get involved.

License

Volition is released under the Apache-2.0 License. See [LICENSE](LICENSE) for details.

License Change Notice

Important: This project was previously released under the MIT License. As of 2025-03-28, the license has been changed to Apache License 2.0.

The Apache 2.0 license provides additional patent protections not present in the MIT license. This change was made to better protect contributors and users
of this project while maintaining the open source nature of the code.

Any copies or forks of this repository created prior to this change date would still be under the MIT license terms. All new contributions and usage will be
governed by the Apache License 2.0.