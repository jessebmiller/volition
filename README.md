# Volition

<div align="center">

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![Rust](https://img.shields.io/badge/language-Rust-orange)
![Status](https://img.shields.io/badge/status-experimental-yellow)

</div>

## 🌟 Overview

Volition is an open-source, Rust-specialized AI coding assistant that empowers developers to interact with their codebase through natural language. Built with a focus on flexibility and extensibility, Volition supports multiple AI providers and interaction strategies.

> ⚠️ **Early Development Warning**: Volition is in the experimental stage. It currently lacks comprehensive safety measures and is intended for development use only. Use at your own risk.

## ✨ Key Features

- **Rust Specialized**: Optimized for Rust development workflows with deep understanding of Rust idioms and patterns
- **Multi-Provider Support**: Compatible with various LLM providers (Google, OpenAI, Ollama, etc.)
- **Strategy Pattern Architecture**: Flexible interaction models including PlanExecute and CompleteTask strategies
- **MCP (Model Context Protocol) First**: Native support for the emerging MCP standard
- **CLI + Library Separation**: Use as a command-line tool or integrate the core library into your applications
- **Free and Open Source**: Licensed under Apache 2.0

## 📦 Installation

### Prerequisites

- Rust toolchain (1.70+)
- For non-local models: API keys for your preferred provider(s)

### From Source

```bash
# Clone the repository
git clone https://github.com/username/volition.git
cd volition

# Build the project
cargo build --release

# Optional: Install the binary
cargo install --path volition-cli
```

## 🚀 Quick Start

### Configuration

Create a `Volition.toml` file in your project root:

```toml
# Default AI provider to use
default_provider = "gemini"

# System prompt that defines Volition's behavior
system_prompt = """
You are Volition, an AI-powered Rust engineering assistant.
Your goal is to help the user understand, modify, and improve their Rust codebase.
Always follow Rust best practices and idioms.
"""

# --- AI Model Providers ---
[providers]
  [providers.gemini]
  type = "gemini"
  api_key_env_var = "GEMINI_API_KEY"
  [providers.gemini.model_config]
    model_name = "gemini-1.5-pro"
    endpoint = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent"
    parameters = { temperature = 0.7 }

# --- MCP Tool Servers ---
[mcp_servers]
  [mcp_servers.filesystem]
  command = "/path/to/volition-filesystem-server"
  [mcp_servers.git]
  command = "/path/to/volition-git-server"
  [mcp_servers.search]
  command = "/path/to/volition-search-server"
  [mcp_servers.shell]
  command = "/path/to/volition-shell-server"

# --- Strategy Configurations ---
[strategies]
  [strategies.plan_execute]
  planning_provider = "gemini"
  execution_provider = "gemini"
```

### Usage

Interactive mode:

```bash
volition
```

Single task mode:

```bash
volition --task "Implement proper error handling in src/main.rs"
```

Enable verbose logging:

```bash
volition -v  # Info level
volition -vv  # Debug level
volition -vvv  # Trace level
```

## 🧩 Architecture

Volition is built with a modular architecture:

- **volition-cli**: Command-line interface
- **volition-agent-core**: Core agent library with providers, strategies, and tools
- **volition-*-server**: MCP server implementations for different tool domains

## 🛠️ Development

### Building the Project

```bash
# Build all components
cargo build

# Run tests
cargo test

# Run with development features
cargo run -p volition-cli
```

### Server Components

Start the MCP servers manually for development:

```bash
cargo run -p volition-filesystem-server
cargo run -p volition-git-server
cargo run -p volition-search-server
cargo run -p volition-shell-server
```

## 🤝 Contributing

Contributions are welcome! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📜 License

Distributed under the Apache License 2.0. See [LICENSE](LICENSE) for more information.

### License Change Notice
 
 **Important:** This project was previously released under the MIT License. As of 2025-03-28, the license has been changed to Apache License 2.0.
 
 The Apache 2.0 license provides additional patent protections not present in the MIT license. This change was made to better protect contributors and users
 of this project while maintaining the open source nature of the code.
 
 Any copies or forks of this repository created prior to this change date would still be under the MIT license terms. All new contributions and usage will be
 governed by the Apache License 2.0.

## 🔗 Links

- [GitHub Repository](https://github.com/jessebmiller/volition)
- [Issue Tracker](https://github.com/jessebmiller/volition/issues)
- [MCP Protocol Specification](https://github.com/modelcontextprotocol/mcp)
