# turboCommit devcontainer agent playbook

- Purpose: deterministically bring up the provided devcontainer, build the Rust CLI, and run it against prepared test repositories entirely inside the container.
- Use absolute paths and `devcontainer exec` to avoid host/container confusion.

## Constants
- ABS_WORKSPACE: `/home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9`
- In-container mount: `/workspace` (bind-mounted from ABS_WORKSPACE)
- Test repos (inside container):
  - Git: `/tmp/testenv/git-repo` (staged changes present)
  - Jj: `/tmp/testenv/jj-repo` (uncommitted changes present)

## Prerequisites
- Docker daemon available
- `devcontainer` CLI on PATH

## Bring up the container (idempotent)
```bash
devcontainer up --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 --log-level debug
```
Expected:
- Image `mcr.microsoft.com/devcontainers/rust:latest`
- Post-create runs `.devcontainer/setup-test-repos.sh` (installs `jj`, creates test repos)

## Exec pattern (always use)
```bash
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc '<commands>'
```
Notes:
- Use `bash -lc` so PATH and login shell env are loaded.
- Chain with `&&` to fail fast.

## Verify toolchain inside container
```bash
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'rustc --version && cargo --version && git --version && jj --version'
```

## Build the CLI (workspace is /workspace)
```bash
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'cd /workspace && cargo build --quiet'
```
Binary path after build: `/workspace/target/debug/turbocommit`

## Sanity check (no API call)
- Use the correct flag: `--check-version`
```bash
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc '/workspace/target/debug/turbocommit --check-version || cargo run --quiet -- --check-version'
```

## Run against prepared Git repo (works end-to-end)
- Provide API key via env var (preferred) or `--api-key` flag. CLI reads `OPENAI_API_KEY`.
```bash
# Using env var
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'export OPENAI_API_KEY=sk-REDACTED && cd /tmp/testenv/git-repo && /workspace/target/debug/turbocommit --print-once -n 1 --model gpt-4o-mini'

# Using flag
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'cd /tmp/testenv/git-repo && /workspace/target/debug/turbocommit --print-once -n 1 --model gpt-4o-mini --api-key sk-REDACTED'
```
Notes:
- First run will create config at `/home/vscode/.turbocommit.yaml` in the container.
- With an invalid key, expect HTTP 401 from OpenAI (confirms plumbing).

## Run against prepared Jj repo (current limitation)
- Container’s `jj` is 0.34.x; `jj status --porcelain` is not supported and the CLI will fail early with:
  - `error: unexpected argument '--porcelain'`
- Until the CLI is adjusted, skip Jj-based runs or use the Git repo for validation.
```bash
# Example invocation (will fail due to jj flag)
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'export OPENAI_API_KEY=sk-REDACTED && cd /tmp/testenv/jj-repo && /workspace/target/debug/turbocommit --print-once -n 1 --model gpt-4o-mini --enable-reasoning'
```

## Reusable sequences
```bash
# Build then version-check
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'cd /workspace && cargo build --quiet && /workspace/target/debug/turbocommit --check-version'

# Full Git run with explicit env
devcontainer exec --workspace-folder /home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9 bash -lc 'export OPENAI_API_KEY=sk-REDACTED && cd /tmp/testenv/git-repo && /workspace/target/debug/turbocommit --print-once -n 1 --model gpt-4o-mini'
```

## Pitfalls
- Use `--check-version` (not `--check-version-only`).
- Always `cd /workspace` before running `cargo`.
- Use absolute paths everywhere.
- One-time "No configuration file found" message on first run is expected.

## Paths summary
- Host workspace: `/home/dikka/projs/turbocommit/cursor/unified-vcs-description-handler-68c9`
- In-container workspace: `/workspace`
- Binary: `/workspace/target/debug/turbocommit`
- Config (in container): `/home/vscode/.turbocommit.yaml`
- Test repos: `/tmp/testenv/git-repo`, `/tmp/testenv/jj-repo`
