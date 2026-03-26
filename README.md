# turboCommit

![Crates.io](https://img.shields.io/crates/v/turbocommit)
![Crates.io](https://img.shields.io/crates/d/turbocommit)
![Crates.io](https://img.shields.io/crates/l/turbocommit)

A CLI tool that uses OpenAI `gpt-5.4` to generate high-quality conventional commit messages from staged changes in Git and Jujutsu (JJ) repositories.

**Version 3.0 standardizes on GPT-5.4.**
**Legacy GPT-5.1 and multi-model compatibility paths have been removed.**

## Features

- `gpt-5.4` only, with no legacy model fallbacks
- Default system prompt rewritten for GPT-5.4-era instruction following
- Conventional commit suggestions from staged Git or JJ changes
- Interactive selection from multiple suggestions
- Direct edit and AI revision loop before commit
- Reasoning effort controls: `none`, `low`, `medium`, `high`
- Verbosity controls: `low`, `medium`, `high`
- Structured JSON outputs for stable multi-suggestion parsing
- Debug logging for requests, responses, token usage, and timing
- YAML configuration via `~/.turbocommit.yaml`

## Installation

```bash
cargo install turbocommit
```

Optional shell alias:

```bash
alias tc='turbocommit'
```

## Usage

1. Stage your changes.

```bash
git add .
```

2. Generate commit suggestions.

```bash
turbocommit
```

After generation you can select a suggestion, edit it, ask for revisions, or commit it directly.

### Options

`turboCommit` now supports only `gpt-5.4`.

- `-n <number>`: number of commit message suggestions to generate, default `3`
- `-m, --model <model>`: model to use, must be `gpt-5.4`
- `-e, --reasoning-effort <level>`: `none`, `low`, `medium`, `high`
- `-v, --verbosity <level>`: `low`, `medium`, `high`
- `-d, --debug`: print request and usage details
- `--debug-file <path>`: write detailed debug logs to a file, or `-` for stdout
- `--auto-commit`: commit automatically using the generated message
- `--amend`: regenerate the last commit message from the last commit diff
- `--api-key <key>`: provide API key directly
- `--api-endpoint <url>`: override the API endpoint
- `-c, --config <path>`: load a non-default config file
- `-r, --revision <rev>`: select a JJ revision to describe
- `--rw`: toggle JJ rewrite mode

### Reasoning

`gpt-5.4` supports configurable reasoning effort.

```bash
turbocommit -m gpt-5.4
turbocommit --reasoning-effort high -m gpt-5.4
turbocommit --reasoning-effort none -m gpt-5.4
```

### Verbosity

```bash
turbocommit --verbosity low -m gpt-5.4
turbocommit --verbosity medium -m gpt-5.4
turbocommit --verbosity high -m gpt-5.4
```

### Debugging

```bash
turbocommit -d
turbocommit --debug-file debug.log
turbocommit --debug-file -
```

Debug logs include request parameters, API responses or errors, token counts, and elapsed time.

## Pricing

The tool is now documented against OpenAI's current GPT-5.4 API pricing.

- `gpt-5.4` input: `$2.50 / 1M tokens`
- `gpt-5.4` cached input: `$0.25 / 1M tokens`
- `gpt-5.4` output: `$15.00 / 1M tokens`

Notes:

- `gpt-5.4-pro` exists, but this CLI does not target it.
- OpenAI documents a 1.05M context window for `gpt-5.4`, with higher pricing for prompts above 272K input tokens.
- This project continues to use `v1/chat/completions`, which OpenAI documents as supported for `gpt-5.4`.

## Configuration

`turboCommit` creates `~/.turbocommit.yaml` on first run.

Example:

```yaml
model: "gpt-5.4"
default_number_of_choices: 3
reasoning_effort: "low"
verbosity: "medium"
disable_auto_update_check: false
api_endpoint: "https://api.openai.com/v1/chat/completions"
api_key_env_var: "OPENAI_API_KEY"
```

Important:

- `model` must be `gpt-5.4`
- empty `system_msg` values are rejected and the default prompt is shown in the validation error

### Multiple Config Files

```bash
turbocommit -c ./local-config.yaml
turbocommit -c ~/.turbocommit-azure.yaml
turbocommit
```

## Amend Flow

Use `--amend` when you want to improve the last commit message without staged changes.

```bash
git status
turbocommit --amend
turbocommit --amend --auto-commit
```

Constraints:

- no staged changes when using `--amend`
- the tool analyzes the previous commit diff only

## Git Hooks and JJ

Recommended workflow:

1. Stage and commit normally.
2. Fix any hook failures.
3. Re-stage fixes if needed.
4. Use `turbocommit --amend` after checks pass if you want a better message.

## Dev Container Test Environment

A disposable Dev Container is included for validating Git and JJ integration without touching real repositories.

```bash
devcontainer up --workspace-folder .
devcontainer exec --workspace-folder . bash
```

## Contributing

Issues and pull requests are welcome.

## License

Licensed under MIT. See [LICENSE](LICENSE).
