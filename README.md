# turbocommit

![Crates.io](https://img.shields.io/crates/v/turbocommit)
![Crates.io](https://img.shields.io/crates/d/turbocommit)
![Crates.io](https://img.shields.io/crates/l/turbocommit)

[`turbocommit` is a CLI tool written in Rust](https://crates.io/crates/turbocommit) that generates commit messages in accordance with the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification. It uses the git diff to create commit messages that accurately reflect the changes made to a repository.

## Installation

turbocommit can be easily installed with Cargo, Rust's package manager. Simply run the following command:

```bash
cargo install turbocommit
```

Please note that in order to use turbocommit, you will need to set the `OPENAI_API_KEY` environment variable. This API key is required to use the OpenAI `gpt-3.5-turbo16k` language model, which is used by turbocommit to generate commit messages.

## Usage

When you have staged some changes, you can run the `turbocommit` (I recommend making a `tc` symlink)

![](example.gif)

### Generating Conventional Commits with `turbocommit`

<!-- START TABLE HERE -->
| Short | Long         | Description                                 |     Default      |
| ----- | ------------ | ------------------------------------------- | :--------------: |
| -n    |              | Number of choices to generate               |        1         |
| -m    | --model      | Model to use                                | gpt-4o-mini      |
| -d    | --dry-run    | Dry run. Will not ask AI for completions    |                  |
| -p    | --print-once | Will not print tokens as they are generated |                  |
| -t    |              | Temperature (t \|0.0 < t < 2.0\|)           |       1.0        |
| -f    |              | Frequency penalty (f \|-2.0 < f < 2.0\|)    |       0.0        |
<!-- END TABLE HERE -->

### Available Models

Please look at the help `turbocommit --help` for the most up-to-date list of models.


### Handling Long `git diff`

In some cases, the `git diff` for staged changes may be too long to fit within the 16384 (or 4096) token limit enforced by the language model, which `turbocommit` uses to generate commit messages. When this happens, `turbocommit` will prompt you with a message indicating that the `git diff` is too long.

To address this, `turbocommit` provides a list of all staged files and ask you to select any number of them. The tool will then generate a new `git diff` that includes only the changes from the selected files. If the resulting `git diff` plus the system prompt is still too long, `turbocommit` will repeat the process until the `git diff` is short enough to be processed by the AI model.

This allows you to generate conventional commit messages with `turbocommit` while ensuring that the `git diff` is short enough to be processed by the AI model.

### Getting Help with `turbocommit`

To get help with using `turbocommit`, you can use the `-h` or `--help` option

```bash
$ turbocommit --help
```

This will display the help message with information on how to use the tool.
