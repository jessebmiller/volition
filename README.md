# Volition

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
![Rust](https://img.shields.io/badge/rust-stable-orange.svg)
[![Stars](https://img.shields.io/github/stars/jessebmiller/volition?style=social)](https://github.com/jessebmiller/volition/stargazers)

**Volition is a contextually-aware AI assistant that supercharges your Rust development workflow.**

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

Volition is an open-source CLI tool that brings the power of advanced AI assistants directly into your development workflow. Unlike many existing tools that either generate new code in isolation or offer general programming advice, Volition deeply integrates with your codebase and development tools to provide contextually-aware assistance.

> 💡 **Built for Rust developers by Rust developers** - Volition understands Rust idioms and best practices.

```ignore
$ volition "Find references to RuntimeConfig across the codebase and explain how it's used"

Volition - AI Assistant
Searching for references to RuntimeConfig in the codebase...

I found 12 references to RuntimeConfig across 5 files. The struct is defined in src/config.rs
and represents configuration loaded from Volition.toml plus environment variables.

Key usage patterns:
1. In src/api.rs: Used to access API credentials and retrieve model configuration
2. In src/tools/mod.rs: Passed to tool implementations that need project context
3. In src/main.rs: Loaded at startup via load_runtime_config()

The struct centralizes access to:
- System prompts
- API credentials
- Model configurations
- Project root path

Need more details or would you like me to explain a specific aspect?
```

## Key Features

- **Tool-based Interaction**: Execute shell commands, read/write files, search codebases, and more through a unified interface.
- **Multiple Model Support**: Works with OpenAI, Google Gemini, and Ollama models through a consistent interface.
- **Deep Code Understanding**: Contextually search and modify codebases with precise tools.
- **Git and Cargo Integration**: Safely use git and cargo commands within your AI workflow.
- **Conversation Recovery**: Automatically saves conversation state for recovery if interrupted.
- **Privacy Focused**: Your code stays on your machine. Only queries are sent to API providers.
- **Advanced Problem Solving**: Implements strategies like simulated annealing for complex problems.

## Installation

### Prerequisites

- **Rust**: [Install Rust](https://www.rust-lang.org/tools/install) (required to build from source)
- **ripgrep**: Required for code search functionality
  - macOS: `brew install ripgrep`
  - Ubuntu/Debian: `apt install ripgrep`
  - Windows: `scoop install ripgrep` or `choco install ripgrep`

### Build from Source

```bash
# Clone the repository
git clone https://github.com/jessebmiller/volition.git
cd volition

# Build the project
cargo build --release

# Optional: Add to your PATH
cp target/release/volition ~/.local/bin/   # Unix
# or
copy target\release\volition.exe %USERPROFILE%\bin\   # Windows
```

## Configuration

1. Create a `Volition.toml` configuration file (see example below)
2. Set your API key as an environment variable: `export API_KEY=your_api_key_here`

Example `Volition.toml`:

```toml
# System prompt defining the AI's role and capabilities
system_prompt = '''
You are Volition, an AI-powered software engineering assistant specializing in code analysis, refactoring, and product engineering.
...
'''

# Selected model (must match a key in the [models] section)
selected_model = "gemini-2-5-pro"

# Available models
[models]
  [models.gpt-4o]
  model_name = "gpt-4o"
  endpoint = "https://api.openai.com/v1/chat/completions"
  parameters = { temperature = 0.2 }

  [models.gemini-2-5-pro]
  model_name = "gemini-2.5-pro-exp-03-25"
  endpoint = "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
  parameters = { temperature = 0.7 }
```

## Usage

### Basic Usage

```bash
# Start an interactive session
volition "Help me understand the architecture of this codebase"

# Increase verbosity for debugging
volition -v "Refactor the error handling in src/tools/shell.rs"
```

### Advanced Features

Volition gives you access to powerful tools through natural language requests:

- **Code Search**: "Find all usages of the `handle_tool_calls` function"
- **Code Analysis**: "Analyze the error handling pattern in this project"
- **Refactoring**: "Extract the API response parsing logic into a separate function"
- **Git Integration**: "Commit my changes with a descriptive message"

## Philosophy

Volition is committed to remaining **free and open-source forever**. While some services may be built around it in the future (such as hosted vector databases for improved context), the core tool will always be free to use, with users only paying for their own API costs.

We believe AI assistants should:
1. **Augment workflows** rather than replace developers
2. **Respect privacy** by keeping code on your machine
3. **Integrate deeply** with existing development tools
4. **Remain transparent** in how they work

## Community

- **GitHub Discussions**: [Join the conversation](https://github.com/jessebmiller/volition/discussions)
- **Discord**: [Join our community](https://discord.gg/example)
- **Twitter**: Follow [@VolitionAI](https://twitter.com/example) for updates

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for more details on how to get involved.

## License

Volition is released under the MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">
<i>Volition: Your AI-powered software engineering partner.</i>
</div>
