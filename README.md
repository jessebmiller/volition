# Volition

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/volition-cli.svg)](https://crates.io/crates/volition-cli)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)
![Rust](https://img.shields.io/badge/language-Rust-orange)
![Status](https://img.shields.io/badge/status-experimental-yellow)

</div>

## 🌟 Overview

Volition is an open-source, Rust-specialized AI coding assistant that
empowers developers to interact with their codebase through natural
language. Built with a focus on flexibility and extensibility,
Volition supports multiple AI providers and interaction strategies.

> ⚠ **Early Development Warning**: Volition is in the experimental
  stage. It currently lacks comprehensive safety measures and is
  intended for development use only. Use at your own risk.

## ✨ Key Features

- **Rust Specialized**: Optimized for Rust development workflows with deep understanding of Rust idioms and patterns
- **Multi-Provider Support**: Compatible with various LLM providers (Google, OpenAI, Ollama, etc.)
- **Strategy Pattern Architecture**: Flexible interaction models including PlanExecute and CompleteTask strategies
- **MCP (Model Context Protocol) First**: Native support for the emerging MCP standard
- **CLI + Library Separation**: Use as a command-line tool or integrate the core library into your applications
- **Free and Open Source**: Licensed under Apache 2.0

## 📦 Installation

### Prerequisites

- Rust toolchain (1.70+ recommended)
- For non-local models: API keys for your preferred provider(s), set as environment variables (e.g., `GEMINI_API_KEY`).

### From crates.io (Recommended)

Once published, you can install the command-line tool directly from crates.io:

```bash
cargo install volition-cli --locked
```

This will make the volition command and the associated
volition-*-server binaries available in your environment (typically in
$HOME/.cargo/bin, ensure this directory is in your system's
PATH). Using --locked ensures you use the exact dependency versions
tested for the release.

### From Source

```bash
# Clone the repository
git clone https://github.com/jessebmiller/volition.git
cd volition

# Build the project
cargo build --release

# Optional: Install the binary and servers locally from source
cargo install --path volition-cli
```

## 🚀 Quick Start

### Configuration

Create a `Volition.toml` file in your project root:

```toml
default_provider = "gemini"

system_prompt = """
You are Volition, an AI-powered Rust engineering assistant.
Your goal is to help the user understand, modify, and improve their Rust codebase.
Always follow Rust best practices and idioms.
"""

[providers]
  [providers.gemini]
  type = "gemini"
  # Make sure the environment variable below is set in your envionment or a .env file
  api_key_env_var = "GEMINI_API_KEY"
  [providers.gemini.model_config]
    model_name = "gemini-1.5-pro"
    endpoint = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro:generateContent"
    parameters = { temperature = 0.7 }

[mcp_servers]
  # See note below on MCP Server Paths for more information
  [mcp_servers.filesystem]
  command = "volition-filesystem-server"
  [mcp_servers.git]
  command = "volition-git-server"
  [mcp_servers.search]
  command = "volition-search-server"
  [mcp_servers.shell]
  command = "volition-shell-server"


[strategies]
  [strategies.plan_execute]
  planning_provider = "gemini"
  execution_provider = "gemini"
```

#### Note on MCP Server Paths:

- The command paths under [mcp_servers] tell Volition how to launch the tools it needs (like filesystem access, git operations, etc.).
- If you installed using cargo install volition-cli, the server binaries (volition-filesystem-server, volition-git-server, etc.) are placed alongside the main volition binary, typically in $HOME/.cargo/bin/.
- If $HOME/.cargo/bin is in your system's PATH environment variable (which is common for Rust setups), you can often just use the command name directly (e.g., command = "volition-filesystem-server") as shown in the example above.
- If it's not in your PATH or you need specific paths, provide the full absolute path (e.g., command = "/path/to/your/.cargo/bin/volition-filesystem-server").
- If running from a source checkout (using cargo run), you would use relative paths to the build output, like command = "target/debug/volition-filesystem-server".
- Future versions may include automatic server discovery or management to simplify this setup.

### Usage

Ensure your API keys are set as environment variables or are set in a
.env file. These variable names are configured in Volition.toml (e.g.,
export GEMINI_API_KEY). Volition.toml should be located in the current
directory or a parent directory.

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
- **volition-core**: Core agent library with providers, strategies, and tools
- (Individual server crates also exist in the workspace but are built/installed via volition-cli)

## 🛠 Development

### Building the Project

```bash
# Build all components
cargo build

# Run tests
cargo test

# Run with development features
cargo run -p volition-cli -- [ARGS]
```

### Server Components

The MCP servers are separate binaries. When running the CLI via `cargo
run -p volition-cli`, it may not automatically find the server
binaries unless they are built and their paths are configured
correctly in a development `Volition.toml` (e.g., pointing to
`target/debug/volition-filesystem-server`).

You can also run servers manually for testing if needed.

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

 **Important:** This project was previously released under the MIT
   License. As of 2025-03-28, the license has been changed to Apache
   License 2.0.

 The Apache 2.0 license provides additional patent protections not
 present in the MIT license. This change was made to better protect
 contributors and users of this project while maintaining the open
 source nature of the code.

 Any copies or forks of this repository created prior to this change
 date would still be under the MIT license terms. All new
 contributions and usage will be governed by the Apache License 2.0.

## 🔗 Links

- [Discord](https://discord.gg/TeCzH7Vf6K)
- [GitHub Repository](https://github.com/jessebmiller/volition)
- [Issue Tracker](https://github.com/jessebmiller/volition/issues)
- [MCP Protocol Specification](https://github.com/modelcontextprotocol/mcp)

The author of Volition is looking for full time product and
engineering work. Please contact him at jesse@jessebmiller.com.