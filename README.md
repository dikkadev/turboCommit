# turboCommit

![Crates.io](https://img.shields.io/crates/v/turbocommit)
![Crates.io](https://img.shields.io/crates/d/turbocommit)
![Crates.io](https://img.shields.io/crates/l/turbocommit)

A powerful CLI tool that leverages OpenAI's GPT models to generate high-quality, conventional commit messages from your staged changes.

## Features

- 🤖 Uses OpenAI's GPT models to analyze your staged changes
- 📝 Generates conventional commit messages that follow best practices
- 🎯 Interactive selection from multiple commit message suggestions
- ✏️ Edit messages directly or request AI revisions
- 🧠 Advanced reasoning mode for enhanced AI interactions
- 🔍 Comprehensive debugging capabilities with file or stdout logging
- ⚡ Streaming responses for real-time feedback
- 🔄 Auto-update checks to keep you on the latest version
- 🎨 Beautiful terminal UI with color-coded output
- ⚙️ Configurable settings via YAML config file

## Installation

```bash
cargo install turbocommit
```

Pro tip: Add an alias to your shell configuration for quicker access:
```bash
# Add to your .bashrc, .zshrc, etc.
alias tc='turbocommit'
```

## Usage

1. Stage your changes:
```bash
git add .  # or stage specific files
```

2. Generate commit messages:
```bash
turbocommit  # or 'tc' if you set up the alias
```

After generating commit messages, you can:
- Select your preferred message from multiple suggestions
- Edit the message directly before committing
- Request AI revisions with additional context or requirements
- Commit the message once you're satisfied

### Options

- `-n <number>` - Number of commit message suggestions to generate
- `-t <temperature>` - Temperature for GPT model (0.0 to 2.0) (no effect in reasoning mode)
- `-f <frequency_penalty>` - Frequency penalty (-2.0 to 2.0)
- `-m <model>` - Specify the GPT model to use
- `-r, --enable-reasoning` - Enable support for models with reasoning capabilities (like o-series)
- `--reasoning-effort <level>` - Set reasoning effort for supported models (low/medium/high, default: medium)
- `-d, --debug` - Show basic debug info in console
- `--debug-file <path>` - Write detailed debug logs to file (use '-' for stdout)
- `--auto-commit` - Automatically commit with the generated message
- `--api-key <key>` - Provide API key directly
- `--api-endpoint <url>` - Custom API endpoint URL
- `-p, --print-once` - Disable streaming output

#### Reasoning Mode
When using models that support reasoning capabilities (like OpenAI's o-series), this mode enables their built-in reasoning features. These models are specifically designed to analyze code changes and generate commit messages with their own reasoning process.

Example usage:
```bash
turbocommit -r -m o3-mini -n 1  # Enable reasoning mode with default effort
turbocommit -r --reasoning-effort high -m o3-mini -n 1  # Specify reasoning effort
```

#### Debugging
Debug output helps troubleshoot API interactions:
```bash
turbocommit -d  # Basic info to console
turbocommit --debug-file debug.log  # Detailed logs to file
turbocommit --debug-file -  # Detailed logs to stdout
```

The debug logs include:
- Request details (model, tokens, parameters)
- API responses and errors
- Timing information
- Full request/response JSON (in file mode)

### Model-Specific Notes

Different models have different capabilities and limitations:

#### O-Series Models (e.g., o3-mini)
- Support reasoning mode
- Do not support temperature/frequency parameters
- May not support multiple choices (`-n`)
- Optimized for specific tasks

#### Standard GPT Models
- Support all parameters
- Multiple choices available
- Temperature and frequency tuning
- Standard reasoning capabilities

For more options, run:
```bash
turbocommit --help
```

## Configuration

turboCommit creates a config file at `~/.turbocommit.yaml` on first run. You can customize:

- Default model
- API endpoint
- Temperature and frequency penalty
- Number of suggestions
- System message prompt
- Auto-update checks
- Reasoning mode defaults
- And more!

Example configuration:
```yaml
model: "gpt-4"
default_temperature: 1.0
default_frequency_penalty: 0.0
default_number_of_choices: 3
enable_reasoning: true
reasoning_effort: "medium"
disable_print_as_stream: false
disable_auto_update_check: false
```

### Multiple Config Files

You can maintain multiple configuration files for different use cases (e.g., different providers or environments) and specify which one to use with the `-c` or `--config` option:

```bash
# Use a local config file
turbocommit -c ./local-config.yaml

# Use a different provider's config
turbocommit -c ~/.turbocommit-azure.yaml

# Use the default config
turbocommit  # uses ~/.turbocommit.yaml
```

Each config file follows the same format as shown above. This allows you to easily switch between different configurations without modifying the default config file.

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

## License

Licensed under MIT - see the [LICENSE](LICENSE) file for details.