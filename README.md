# turboCommit

![Crates.io](https://img.shields.io/crates/v/turbocommit)
![Crates.io](https://img.shields.io/crates/d/turbocommit)
![Crates.io](https://img.shields.io/crates/l/turbocommit)

A CLI tool that uses the GPT-6 family to write Conventional Commit messages from Git and Jujutsu (JJ) changes. GPT-6 Luna is the default for its speed and low cost; Sol and Astra are available for harder changes.

## Features

- GPT-6 Luna by default, with Sol and Astra available via `--model`
- One suggestion by default; request more with `-n`
- Prompt tuned for commit intent, JJ descriptions, and concise output
- Conventional Commit suggestions from staged Git changes or a JJ revision
- Interactive review, editing, and revision when `--auto-commit` is not set
- Reasoning effort controls: `none`, `low`, `medium`, `high`, `xhigh`, `max` (Astra starts at `low`)
- Verbosity controls: `low`, `medium`, `high`
- Structured JSON outputs for stable multi-suggestion parsing
- Debug logging for requests, responses, token usage, and timing
- YAML configuration via `~/.turbocommit.yaml`

## Installation

```bash
cargo install --git https://github.com/dikkadev/turboCommit
```

For the latest version published to crates.io, use `cargo install turbocommit`. The published release may lag behind the repository.

Optional shell alias:

```bash
alias tc='turbocommit'
```

## Usage

1. For Git, stage your changes. JJ uses the selected revision's changes directly.

```bash
git add .
```

2. Generate one suggestion and review it.

```bash
turbocommit
```

The interactive flow lets you accept, edit, revise, or abort. For the common automatic flow, use `turbocommit --auto-commit`. With Git, it creates a commit; with JJ, it sets the selected revision's description. It always requests one suggestion, regardless of `-n` order.

```bash
# Describe the current JJ change without prompts
turbocommit --auto-commit

# Describe a specific JJ revision, using its current description as a hint
turbocommit --auto-commit --revision <rev> --rw
```

### Options

- `-n <number>`: number of suggestions, default `1` in interactive mode
- `-m, --model <model>`: `gpt-6-luna` (default), `gpt-6-sol`, or `gpt-6-astra`
- `-e, --reasoning-effort <level>`: `none`, `low` (default), `medium`, `high`, `xhigh`, `max`; Astra does not support `none`
- `-v, --verbosity <level>`: `low`, `medium`, `high`
- `-d, --debug`: print request and usage details
- `--debug-file <path>`: write detailed debug logs to a file, or `-` for stdout
- `-a, --auto-commit`: write the generated Git commit or JJ description without interactive review
- `--amend`: regenerate the last Git commit message from its diff
- `--api-key <key>`: provide API key directly
- `--api-endpoint <url>`: override the API endpoint
- `-c, --config <path>`: load a non-default config file
- `-r, --revision <rev>`: select a JJ revision to describe
- `--rw`: toggle JJ rewrite mode, which passes the current description to the model as context

### Reasoning

Reasoning effort defaults to `low` for fast commit generation. Use a higher level for ambiguous or broad diffs.

```bash
turbocommit --auto-commit
turbocommit --reasoning-effort high -m gpt-6-sol
turbocommit --reasoning-effort none -m gpt-6-luna
```

### Verbosity

```bash
turbocommit --verbosity low
turbocommit --verbosity medium
turbocommit --verbosity high
```

### Debugging

```bash
turbocommit -d
turbocommit --debug-file debug.log
turbocommit --debug-file -
```

Debug logs include request parameters, API responses or errors, token counts, and elapsed time.

## Pricing

Standard [GPT-6 Luna API pricing](https://developers.openai.com/api/docs/models/gpt-6-luna) per 1M text tokens:

- Input: $0.10
- Cached input: $0.01
- Output: $0.50

All three supported models have a 1.05M token context window. OpenAI charges higher rates for prompts above 272K input tokens; see the [model pages](https://developers.openai.com/api/docs/models). The CLI uses `v1/chat/completions`, which supports these models and the current structured JSON response format. [OpenAI recommends Responses for new text generation applications](https://developers.openai.com/api/docs/guides/text); this CLI keeps its working Chat Completions integration and configurable compatible endpoint.

## Configuration

`turboCommit` creates `~/.turbocommit.yaml` on first run.

Example:

```yaml
model: "gpt-6-luna"
default_number_of_choices: 1
reasoning_effort: "low"
verbosity: "medium"
disable_auto_update_check: false
api_endpoint: "https://api.openai.com/v1/chat/completions"
api_key_env_var: "OPENAI_API_KEY"
```

Omit `system_msg` to use the current built-in prompt. A custom nonempty `system_msg` overrides it. New generated config files omit the default prompt, so future prompt improvements apply automatically. Existing config files created by version 3.x are upgraded in memory from the GPT-5.4 defaults to Luna, one suggestion, and the new prompt. Custom prompts and nondefault suggestion counts are preserved. This migration does not rewrite your config file; edit the stored model or remove the old default prompt when you want the file itself to reflect the new settings.

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

## Git Hooks

Recommended workflow:

1. Stage and commit normally.
2. Fix any hook failures.
3. Re-stage fixes if needed.
4. Use `turbocommit --amend` after checks pass if you want a better message.

JJ does not use Git staging. Run `turbocommit --auto-commit` in a JJ workspace to describe `@`, or pass `--revision` to describe another change. The command updates the description; it does not create a new JJ change.

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
