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
- `-t <temperature>` - Temperature for GPT model (0.0 to 2.0)
- `-f <frequency_penalty>` - Frequency penalty (-2.0 to 2.0)
- `-m <model>` - Specify the GPT model to use
- `--auto-commit` - Automatically commit with the generated message
- `--api-key <key>` - Provide API key directly
- `--api-endpoint <url>` - Custom API endpoint URL
- `-p, --print-once` - Disable streaming output

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
- And more!

## Contributing

Contributions are welcome! Feel free to open issues and pull requests.

## License

Licensed under MIT - see the [LICENSE](LICENSE) file for details.