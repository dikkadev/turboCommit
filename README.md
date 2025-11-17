# turboCommit

![Crates.io](https://img.shields.io/crates/v/turbocommit)
![Crates.io](https://img.shields.io/crates/d/turbocommit)
![Crates.io](https://img.shields.io/crates/l/turbocommit)

A powerful CLI tool that leverages OpenAI's GPT-5.1 models to generate high-quality, conventional commit messages from your staged changes in Git and Jujutsu (JJ) repositories.

**Version 2.0 now supports both Git and Jujutsu (JJ) version control systems!**
**Latest update: Exclusively uses GPT-5.1 models with enhanced reasoning and verbosity controls!**

## Features

- 🤖 **GPT-5.1 Powered** - Exclusively uses the latest GPT-5.1 model family (gpt-5.1, gpt-5.1-codex, gpt-5.1-codex-mini)
- 📝 Generates conventional commit messages that follow best practices
- 🎯 Interactive selection from multiple commit message suggestions
- ✏️ Edit messages directly or request AI revisions
- 🧠 Advanced reasoning mode with configurable effort levels (including 'none' to disable)
- 🗣️ **Verbosity controls** - Configure output detail level (low, medium, high)
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
git add .  # or stage specific files (Git)
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

**Important: turboCommit now only supports GPT-5.1 models (gpt-5.1, gpt-5.1-codex, gpt-5.1-codex-mini)**

- `-n <number>` - Number of commit message suggestions to generate (default: 3)
- `-m <model>` - Specify the GPT-5.1 model to use
  - Supported: `gpt-5.1`, `gpt-5.1-codex`, `gpt-5.1-codex-mini`
- `-r, --enable-reasoning` - Enable reasoning mode (enabled by default for GPT-5.1)
- `-e, --reasoning-effort <level>` - Set reasoning effort level
  - Options: `none` (disable reasoning), `low` (default), `medium`, `high`
- `-v, --verbosity <level>` - Control output verbosity
  - Options: `low`, `medium`, `high`
- `-d, --debug` - Show basic debug info in console
- `--debug-file <path>` - Write detailed debug logs to file (use '-' for stdout)
- `--auto-commit` - Automatically commit with the generated message
- `--amend` - Amend the last commit with the generated message
- `--api-key <key>` - Provide API key directly
- `--api-endpoint <url>` - Custom API endpoint URL
- `-p, --print-once` - Disable streaming output

#### Reasoning Mode
GPT-5.1 models have built-in reasoning capabilities that are enabled by default. These models are specifically designed to analyze code changes and generate commit messages with advanced reasoning.

You can control the reasoning effort level:
```bash
turbocommit -m gpt-5.1                                  # Default reasoning (medium)
turbocommit --reasoning-effort high -m gpt-5.1          # High reasoning effort
turbocommit --reasoning-effort low -m gpt-5.1-codex     # Low reasoning effort
turbocommit --reasoning-effort none -m gpt-5.1          # Disable reasoning features
```

#### Verbosity Control
Control the level of detail in the model's responses:
```bash
turbocommit --verbosity low -m gpt-5.1-codex-mini       # Concise output
turbocommit --verbosity medium -m gpt-5.1               # Balanced output (default)
turbocommit --verbosity high -m gpt-5.1-codex           # Detailed output
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

For more options, run:
```bash
turbocommit --help
```

## Configuration

turboCommit creates a config file at `~/.turbocommit.yaml` on first run. You can customize:

- Default GPT-5.1 model
- API endpoint
- Reasoning effort level
- Verbosity setting
- Number of suggestions
- System message prompt
- Auto-update checks
- Reasoning mode defaults
- And more!

Example configuration:
```yaml
model: "gpt-5.1"  # Must be a GPT-5.1 model: gpt-5.1, gpt-5.1-codex, or gpt-5.1-codex-mini
default_number_of_choices: 3
enable_reasoning: true
verbosity: "medium"  # Options: low, medium, high
disable_print_as_stream: false
disable_auto_update_check: false
api_endpoint: "https://api.openai.com/v1/chat/completions"
api_key_env_var: "OPENAI_API_KEY"
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

### Using turboCommit with --amend

The `--amend` option allows you to change the commit message of your last commit. This is useful when:
- You want to improve the message of your last commit
- You want to fix a typo in your commit message
- You want to add more context to your commit message

Usage:
```bash
# First, make sure you have no staged changes
git status  # Should show no staged changes

# Then use --amend to improve the last commit's message
turbocommit --amend  # This will analyze the last commit's changes and suggest a new message
```

Important Notes:
- When using `--amend`, you must not have any staged changes
- The tool will analyze only the changes from your last commit
- If you want to include new changes in the amended commit:
  1. Either commit them first normally, then amend that commit
  2. Or use `git commit --amend` manually to include them

You can also combine this with auto-commit for a quick message update:
```bash
turbocommit --amend --auto-commit  # Automatically amend with the first generated message
```

### Using turboCommit with Git Hooks and JJ

If your project uses Git hooks (e.g., linters, formatters) or JJ workflows, here's how to use turboCommit effectively:

1. Stage and commit your changes normally:
```bash
# For Git repositories:
git add .
turbocommit

# For JJ repositories:
turbocommit
```

2. If hooks fail:
   - Fix the issues reported by hooks
   - Stage the fixed files (`git add .`)
   - Commit again

3. If you want to improve the commit message after all hooks pass:
```bash
# Make sure you have no staged changes
git status

# Then improve the message
turbocommit --amend  # This will analyze the commit and suggest a better message
```

This workflow ensures that:
- Code quality checks run before the commit
- You can improve the commit message after all checks pass
- The final commit message is high-quality and descriptive

## Dev Container test environment

A disposable Dev Container is provided to safely develop and validate Git + Jujutsu (jj) integration. It includes the Rust toolchain, git and jj, and automatically prepares two throwaway repos inside the container at `/tmp/testenv/`.

- What it’s for: Run experiments and upcoming integration checks in isolation, without touching real repositories.
- How to use it:
  - VS Code/Cursor: Command Palette → "Dev Containers: Reopen in Container".
  - CLI:
    ```bash
    devcontainer up --workspace-folder .
    devcontainer exec --workspace-folder . bash
    ```
