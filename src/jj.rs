use std::path::Path;

use futures::StreamExt;
use jj_lib::backend::TreeValue;
use jj_lib::config::StackedConfig;
use jj_lib::matchers::EverythingMatcher;
use jj_lib::object_id::ObjectId;
use jj_lib::repo::Repo as _;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::Workspace;
use pollster::FutureExt;
use tokio::io::AsyncReadExt;

/// Maximum bytes to check for binary content detection
const BINARY_CHECK_BYTES: usize = 8000;

/// Check if content is binary by looking for null bytes
/// This is the same heuristic used by git
fn is_binary_content(content: &[u8]) -> bool {
    let check_len = content.len().min(BINARY_CHECK_BYTES);
    content[..check_len].contains(&0)
}

/// Represents the VCS type being used
#[derive(Debug, Clone, PartialEq)]
pub enum VcsType {
    Git,
    Jujutsu,
}

/// Detects which VCS is being used in the current directory
pub fn detect_vcs() -> anyhow::Result<VcsType> {
    // Check if we're in a Jujutsu repository
    if Path::new(".jj").exists() {
        return Ok(VcsType::Jujutsu);
    }

    // Check if we're in a git repository
    if Path::new(".git").exists() {
        return Ok(VcsType::Git);
    }

    // Try to load jj workspace to see if we're in a jj repo without .jj directory
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    if let Ok(_workspace) = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    ) {
        return Ok(VcsType::Jujutsu);
    }

    // Try to discover git repository using git2
    if git2::Repository::discover(".").is_ok() {
        return Ok(VcsType::Git);
    }

    Err(anyhow::anyhow!("No supported VCS repository found. Please run this command from within a git or jj repository."))
}

/// Validates that a revision string is safe and not a complex expression
pub fn validate_revision_id(rev: &str) -> anyhow::Result<()> {
    // Simple validation: should not be empty
    if rev.is_empty() {
        return Err(anyhow::anyhow!("Revision cannot be empty"));
    }

    // Reject potentially dangerous characters or complex expressions
    // Allow alphanumeric, hyphens, underscores, dots, colons (for git refs)
    // but reject pipes, semicolons, parentheses, and other shell metacharacters
    if !rev
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
    {
        return Err(anyhow::anyhow!(
            "Revision contains invalid characters. Only alphanumeric characters, hyphens, underscores, dots, and colons are allowed."
        ));
    }

    Ok(())
}

/// Resolves a revision string to a commit ID
/// Supports full hex hashes, commit ID prefixes, and change ID abbreviations (jj display format)
fn resolve_revision_to_commit_id(
    repo: &std::sync::Arc<jj_lib::repo::ReadonlyRepo>,
    rev: &str,
) -> anyhow::Result<jj_lib::backend::CommitId> {
    // First try to parse as a full hex string (full commit ID)
    if let Some(commit_id) = jj_lib::backend::CommitId::try_from_hex(rev) {
        return Ok(commit_id);
    }

    // For non-hex revisions, search through all commits
    let view = repo.view();
    let store = repo.store();
    let mut commit_matches = Vec::new();
    let mut to_visit = Vec::new();
    let mut visited = std::collections::HashSet::new();

    // Normalize the revision string to lowercase once for case-insensitive matching
    let rev_lower = rev.to_lowercase();

    // Start from all visible commit heads
    for head_id in view.heads() {
        if !visited.contains(head_id) {
            to_visit.push(head_id.clone());
        }
    }

    while let Some(commit_id) = to_visit.pop() {
        if !visited.insert(commit_id.clone()) {
            continue;
        }

        if let Ok(commit) = store.get_commit(&commit_id) {
            let change_id = commit.change_id();
            let change_id_reverse_hex = change_id.reverse_hex();
            let commit_id_hex = commit_id.hex();

            // Use case-insensitive comparison for better compatibility
            let commit_id_lower = commit_id_hex.to_lowercase();
            let change_id_lower = change_id_reverse_hex.to_lowercase();

            // Check if commit ID (hex) starts with the given prefix
            if commit_id_lower.starts_with(&rev_lower) {
                commit_matches.push(commit_id.clone());
            }
            // Check if change ID reverse_hex representation starts with the prefix
            // This matches jj's display format for change IDs (e.g., "yqqrnkkn")
            else if change_id_lower.starts_with(&rev_lower) {
                commit_matches.push(commit_id.clone());
            }

            for parent_id in commit.parent_ids() {
                if !visited.contains(parent_id) {
                    to_visit.push(parent_id.clone());
                }
            }
        }
    }

    // Handle results
    if commit_matches.len() == 1 {
        Ok(commit_matches.into_iter().next().unwrap())
    } else if commit_matches.is_empty() {
        Err(anyhow::anyhow!(
            "Invalid revision '{}': could not find matching commit or change. Use 'jj log' to see available commits.",
            rev
        ))
    } else {
        Err(anyhow::anyhow!(
            "Ambiguous revision '{}': matches multiple commits. Use a longer prefix.",
            rev
        ))
    }
}

/// Gets the diff for Jujutsu VCS for specific files
pub fn get_jj_diff_for_files(revision: Option<&str>, files: &[String]) -> anyhow::Result<String> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Resolve revision using jj's index for prefix matching
        let commit_id = resolve_revision_to_commit_id(&repo, rev)?;
        repo.store().get_commit(&commit_id)?
    };

    let tree = commit.tree()?;
    let parent_tree = commit.parent_tree(repo.as_ref())?;

    // Generate proper diff using jj-lib API
    // tree and parent_tree are already MergedTree instances
    let merged_tree = tree;
    let merged_parent_tree = parent_tree;
    let matcher = EverythingMatcher;

    let diff_stream = merged_parent_tree.diff_stream(&merged_tree, &matcher);
    let mut diff_result = String::new();

    // Collect all diff entries and iterate through them
    let entries: Vec<jj_lib::merged_tree::TreeDiffEntry> =
        diff_stream.collect::<Vec<_>>().block_on();
    for entry in entries {
        let path = &entry.path;
        let path_str = path.as_internal_file_string();

        // Only include files that are in the selected list
        if !files.contains(&path_str.to_string()) {
            continue;
        }

        // Get source (before) and target (after) values
        let diff = entry
            .values
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Diff error: {}", e))?;
        let source_value = &diff.before;
        let target_value = &diff.after;

        // Determine change type and generate appropriate diff
        match (source_value.as_resolved(), target_value.as_resolved()) {
            // File deleted (exists in parent, absent in current)
            (Some(Some(TreeValue::File { id: source_id, .. })), Some(None)) => {
                let content = read_file_content(repo.store(), path, source_id).block_on()?;
                // Skip binary files
                if is_binary_content(&content) {
                    diff_result.push_str(&format!(
                        "Binary file {} deleted\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str("deleted file mode 100644\n");
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str("+++ /dev/null\n");

                diff_result.push_str(&format_deletion(&content));
            }

            // File added (absent in parent, exists in current)
            (Some(None), Some(Some(TreeValue::File { id: target_id, .. }))) => {
                let content = read_file_content(repo.store(), path, target_id).block_on()?;
                // Skip binary files
                if is_binary_content(&content) {
                    diff_result.push_str(&format!(
                        "Binary file {} added\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str("new file mode 100644\n");
                diff_result.push_str("--- /dev/null\n");
                diff_result.push_str(&format!("+++ b/{}\n", path_str));

                diff_result.push_str(&format_addition(&content));
            }

            // File modified
            (
                Some(Some(TreeValue::File {
                    id: source_id,
                    executable: source_exec,
                    ..
                })),
                Some(Some(TreeValue::File {
                    id: target_id,
                    executable: target_exec,
                    ..
                })),
            ) if source_id != target_id || source_exec != target_exec => {
                let source_content =
                    read_file_content(repo.store(), path, source_id).block_on()?;
                let target_content =
                    read_file_content(repo.store(), path, target_id).block_on()?;

                // Skip binary files
                if is_binary_content(&source_content) || is_binary_content(&target_content) {
                    diff_result.push_str(&format!(
                        "Binary file {} modified\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));

                if source_exec != target_exec {
                    if *target_exec {
                        diff_result.push_str("old mode 100644\n");
                        diff_result.push_str("new mode 100755\n");
                    } else {
                        diff_result.push_str("old mode 100755\n");
                        diff_result.push_str("new mode 100644\n");
                    }
                }

                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));

                diff_result.push_str(&format_unified_diff(&source_content, &target_content)?);
            }

            // Symlink changes
            (
                Some(Some(TreeValue::Symlink(source_id))),
                Some(Some(TreeValue::Symlink(target_id))),
            ) if source_id != target_id => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str("@@ -1 +1 @@\n");

                let source_target = read_symlink(repo.store(), path, source_id).block_on()?;
                let target_target = read_symlink(repo.store(), path, target_id).block_on()?;
                diff_result.push_str(&format!("-{}\n", source_target));
                diff_result.push_str(&format!("+{}\n", target_target));
            }

            // File type changes (e.g., file to symlink)
            (Some(Some(source)), Some(Some(target)))
                if std::mem::discriminant(source) != std::mem::discriminant(target) =>
            {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str("File type changed\n");
            }

            // No change or unsupported
            _ => {}
        }
    }

    Ok(diff_result)
}

/// Gets the diff for Jujutsu VCS
pub fn get_jj_diff(revision: Option<&str>) -> anyhow::Result<String> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Resolve revision using jj's index for prefix matching
        let commit_id = resolve_revision_to_commit_id(&repo, rev)?;
        repo.store().get_commit(&commit_id)?
    };

    let tree = commit.tree()?;
    let parent_tree = commit.parent_tree(repo.as_ref())?;

    // Generate proper diff using jj-lib API
    // tree and parent_tree are already MergedTree instances
    let merged_tree = tree;
    let merged_parent_tree = parent_tree;
    let matcher = EverythingMatcher;

    let diff_stream = merged_parent_tree.diff_stream(&merged_tree, &matcher);
    let mut diff_result = String::new();

    // Collect all diff entries and iterate through them
    let entries: Vec<jj_lib::merged_tree::TreeDiffEntry> =
        diff_stream.collect::<Vec<_>>().block_on();
    for entry in entries {
        let path = &entry.path;
        let path_str = path.as_internal_file_string();

        // Get source (before) and target (after) values
        let diff = entry
            .values
            .as_ref()
            .map_err(|e| anyhow::anyhow!("Diff error: {}", e))?;
        let source_value = &diff.before;
        let target_value = &diff.after;

        // Determine change type and generate appropriate diff
        match (source_value.as_resolved(), target_value.as_resolved()) {
            // File deleted (exists in parent, absent in current)
            (Some(Some(TreeValue::File { id: source_id, .. })), Some(None)) => {
                let content = read_file_content(repo.store(), path, source_id).block_on()?;
                // Skip binary files
                if is_binary_content(&content) {
                    diff_result.push_str(&format!(
                        "Binary file {} deleted\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str("deleted file mode 100644\n");
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str("+++ /dev/null\n");

                diff_result.push_str(&format_deletion(&content));
            }

            // File added (absent in parent, exists in current)
            (Some(None), Some(Some(TreeValue::File { id: target_id, .. }))) => {
                let content = read_file_content(repo.store(), path, target_id).block_on()?;
                // Skip binary files
                if is_binary_content(&content) {
                    diff_result.push_str(&format!(
                        "Binary file {} added\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str("new file mode 100644\n");
                diff_result.push_str("--- /dev/null\n");
                diff_result.push_str(&format!("+++ b/{}\n", path_str));

                diff_result.push_str(&format_addition(&content));
            }

            // File modified
            (
                Some(Some(TreeValue::File {
                    id: source_id,
                    executable: source_exec,
                    ..
                })),
                Some(Some(TreeValue::File {
                    id: target_id,
                    executable: target_exec,
                    ..
                })),
            ) if source_id != target_id || source_exec != target_exec => {
                let source_content =
                    read_file_content(repo.store(), path, source_id).block_on()?;
                let target_content =
                    read_file_content(repo.store(), path, target_id).block_on()?;

                // Skip binary files
                if is_binary_content(&source_content) || is_binary_content(&target_content) {
                    diff_result.push_str(&format!(
                        "Binary file {} modified\n",
                        path_str
                    ));
                    continue;
                }

                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));

                if source_exec != target_exec {
                    if *target_exec {
                        diff_result.push_str("old mode 100644\n");
                        diff_result.push_str("new mode 100755\n");
                    } else {
                        diff_result.push_str("old mode 100755\n");
                        diff_result.push_str("new mode 100644\n");
                    }
                }

                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));

                diff_result.push_str(&format_unified_diff(&source_content, &target_content)?);
            }

            // Symlink changes
            (
                Some(Some(TreeValue::Symlink(source_id))),
                Some(Some(TreeValue::Symlink(target_id))),
            ) if source_id != target_id => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str("@@ -1 +1 @@\n");

                let source_target = read_symlink(repo.store(), path, source_id).block_on()?;
                let target_target = read_symlink(repo.store(), path, target_id).block_on()?;
                diff_result.push_str(&format!("-{}\n", source_target));
                diff_result.push_str(&format!("+{}\n", target_target));
            }

            // File type changes (e.g., file to symlink)
            (Some(Some(source)), Some(Some(target)))
                if std::mem::discriminant(source) != std::mem::discriminant(target) =>
            {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str("File type changed\n");
            }

            // No change or unsupported
            _ => {}
        }
    }

    Ok(diff_result)
}

/// Read file content from store
async fn read_file_content(
    store: &jj_lib::store::Store,
    path: &jj_lib::repo_path::RepoPath,
    file_id: &jj_lib::backend::FileId,
) -> anyhow::Result<Vec<u8>> {
    let mut reader = store.read_file(path, file_id).await?;
    let mut content = Vec::new();
    reader.read_to_end(&mut content).await?;
    Ok(content)
}

/// Read symlink target
async fn read_symlink(
    store: &jj_lib::store::Store,
    path: &jj_lib::repo_path::RepoPath,
    symlink_id: &jj_lib::backend::SymlinkId,
) -> anyhow::Result<String> {
    let target = store.read_symlink(path, symlink_id).await?;
    Ok(target)
}

/// Format file addition as unified diff
fn format_addition(content: &[u8]) -> String {
    let text = String::from_utf8_lossy(content);
    let lines: Vec<&str> = text.lines().collect();

    let mut output = format!("@@ -0,0 +1,{} @@\n", lines.len());
    for line in lines {
        output.push_str(&format!("+{}\n", line));
    }
    output
}

/// Format file deletion as unified diff
fn format_deletion(content: &[u8]) -> String {
    let text = String::from_utf8_lossy(content);
    let lines: Vec<&str> = text.lines().collect();

    let mut output = format!("@@ -1,{} +0,0 @@\n", lines.len());
    for line in lines {
        output.push_str(&format!("-{}\n", line));
    }
    output
}

/// Generate unified diff format between two file contents
fn format_unified_diff(source: &[u8], target: &[u8]) -> anyhow::Result<String> {
    let source_text = String::from_utf8_lossy(source);
    let target_text = String::from_utf8_lossy(target);

    let source_lines: Vec<&str> = source_text.lines().collect();
    let target_lines: Vec<&str> = target_text.lines().collect();

    let mut output = String::new();

    // Track hunks
    let mut i = 0;
    while i < source_lines.len().max(target_lines.len()) {
        // Find the start of a change region
        if source_lines.get(i).unwrap_or(&"") != target_lines.get(i).unwrap_or(&"") {
            let hunk_start = i;
            let mut hunk_len = 0;
            // Find the end of the contiguous change region
            while i < source_lines.len().max(target_lines.len())
                && source_lines.get(i).unwrap_or(&"") != target_lines.get(i).unwrap_or(&"")
            {
                hunk_len += 1;
                i += 1;
            }
            // Calculate hunk header line numbers and counts
            let src_hunk_start = hunk_start + 1;
            let tgt_hunk_start = hunk_start + 1;
            let src_hunk_count = hunk_len;
            let tgt_hunk_count = hunk_len;
            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                src_hunk_start, src_hunk_count, tgt_hunk_start, tgt_hunk_count
            ));
            // Output the changed lines in the hunk
            for j in hunk_start..(hunk_start + hunk_len) {
                let source_line = source_lines.get(j).unwrap_or(&"");
                let target_line = target_lines.get(j).unwrap_or(&"");
                if !source_line.is_empty() {
                    output.push_str(&format!("-{}\n", source_line));
                }
                if !target_line.is_empty() {
                    output.push_str(&format!("+{}\n", target_line));
                }
            }
        } else {
            i += 1;
        }
    }

    Ok(output)
}

/// Gets the current description for a Jujutsu revision
pub fn get_jj_description(revision: Option<&str>) -> anyhow::Result<Option<String>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Resolve revision using jj's index for prefix matching
        let commit_id = resolve_revision_to_commit_id(&repo, rev)?;
        repo.store().get_commit(&commit_id)?
    };

    let description = commit.description();

    if description.is_empty() {
        Ok(None)
    } else {
        Ok(Some(description.to_string()))
    }
}

/// Sets the description for a Jujutsu revision by rewriting the target commit
pub fn set_jj_description(revision: Option<&str>, description: &str) -> anyhow::Result<()> {
    // Load config with defaults first, then try to load user and repo configs
    let mut config = StackedConfig::with_defaults();

    // Try to load user config from standard locations
    if let Ok(home_dir) = std::env::var("HOME") {
        let user_config_path = std::path::PathBuf::from(home_dir).join(".jjconfig.toml");
        if user_config_path.exists() {
            let _ = config.load_file(jj_lib::config::ConfigSource::User, user_config_path);
        }
    }

    // Try to load repo config
    let repo_config_path = Path::new(".jj/repo/config.toml");
    if repo_config_path.exists() {
        let _ = config.load_file(jj_lib::config::ConfigSource::Repo, repo_config_path);
    }

    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        let commit_id = resolve_revision_to_commit_id(&repo, rev)?;
        repo.store().get_commit(&commit_id)?
    };

    // Get author and committer from original commit, or from user settings if not set
    let author = commit.author();
    let committer = commit.committer();
    
    // Check if author/committer are valid (non-empty name and email)
    let needs_author = author.name.is_empty() || author.email.is_empty();
    let needs_committer = committer.name.is_empty() || committer.email.is_empty();
    
    // Get user signature from settings if needed
    let user_signature = if needs_author || needs_committer {
        let user_name = user_settings.user_name().to_string();
        let user_email = user_settings.user_email().to_string();
        
        // Validate that user settings have name and email configured
        if user_name.is_empty() || user_email.is_empty() {
            return Err(anyhow::anyhow!(
                "Commit author/committer information is missing and user.name/user.email are not configured.\n\
                 Please configure them with:\n\
                 jj config set user.name \"Your Name\"\n\
                 jj config set user.email \"[email protected]\""
            ));
        }
        
        Some(jj_lib::backend::Signature {
            name: user_name,
            email: user_email,
            timestamp: jj_lib::backend::Timestamp::now(),
        })
    } else {
        None
    };
    
    // Start transaction and rewrite the commit with updated description
    let mut tx = repo.start_transaction();
    // Build and write rewritten commit
    {
        let mut_repo = tx.repo_mut();
        let builder = mut_repo.rewrite_commit(&commit);
        
        // Set author and committer, preserving originals if they exist, otherwise using user settings
        let author_sig = if needs_author {
            user_signature.as_ref().unwrap().clone()
        } else {
            author.clone()
        };
        
        let committer_sig = if needs_committer {
            user_signature.as_ref().unwrap().clone()
        } else {
            committer.clone()
        };
        
        builder
            .set_author(author_sig)
            .set_committer(committer_sig)
            .set_description(description.to_string())
            .write()?;
    }
    // Update descendants/refs and working copy if necessary
    tx.repo_mut().rebase_descendants()?;
    // Commit transaction so the change is recorded
    tx.commit("turbocommit: set description")?;
    Ok(())
}

/// Checks if there are any changes for a specific revision in Jujutsu
/// If revision is None, checks the working directory (@)
pub fn has_jj_changes_for_revision(revision: Option<&str>) -> anyhow::Result<bool> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo
            .view()
            .get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Resolve revision using jj's index for prefix matching
        let commit_id = resolve_revision_to_commit_id(&repo, rev)?;
        repo.store().get_commit(&commit_id)?
    };

    // Get the parent tree
    let parent_tree = commit.parent_tree(repo.as_ref())?;
    let current_tree = commit.tree()?;

    // Check if trees are different
    Ok(parent_tree.id() != current_tree.id())
}

/// Gets the list of modified files for Jujutsu
pub fn get_jj_modified_files() -> anyhow::Result<Vec<String>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();

    let workspace = Workspace::load(
        &user_settings,
        Path::new("."),
        &store_factories,
        &working_copy_factories,
    )?;
    let repo = workspace.repo_loader().load_at_head()?;

    // Get the working copy commit
    let wc_commit_id = repo
        .view()
        .get_wc_commit_id(workspace.workspace_name())
        .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
    let wc_commit = repo.store().get_commit(wc_commit_id)?;

    // Get the parent tree
    let parent_tree = wc_commit.parent_tree(repo.as_ref())?;
    let current_tree = wc_commit.tree()?;

    // Compute the diff between parent and current tree
    let mut modified_files = Vec::new();
    let diff_stream = parent_tree.diff_stream(&current_tree, &EverythingMatcher);
    let entries: Vec<jj_lib::merged_tree::TreeDiffEntry> =
        diff_stream.collect::<Vec<_>>().block_on();
    for entry in entries {
        let path = &entry.path;
        // Convert path to string using the internal file string representation
        let path_str = path.as_internal_file_string();
        modified_files.push(path_str.to_string());
    }
    Ok(modified_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary_content_detects_null_bytes() {
        // Binary content with null byte
        let binary_content = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x0D, 0x0A, 0x1A];
        assert!(is_binary_content(&binary_content));
    }

    #[test]
    fn test_is_binary_content_allows_text() {
        // Plain text content
        let text_content = b"Hello, world!\nThis is a text file.\n";
        assert!(!is_binary_content(text_content));
    }

    #[test]
    fn test_is_binary_content_empty() {
        // Empty content should not be considered binary
        let empty_content: Vec<u8> = vec![];
        assert!(!is_binary_content(&empty_content));
    }

    #[test]
    fn test_is_binary_content_utf8_text() {
        // UTF-8 text content with unicode characters
        let utf8_content = "Hello, 世界! 🌍 Unicode text.".as_bytes();
        assert!(!is_binary_content(utf8_content));
    }

    #[test]
    fn test_is_binary_content_null_in_middle() {
        // Null byte in the middle of content
        let mixed_content = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x00, 0x57, 0x6F, 0x72, 0x6C, 0x64];
        assert!(is_binary_content(&mixed_content));
    }

    #[test]
    fn test_is_binary_content_only_checks_first_8000_bytes() {
        // Create content with null byte at position 8001 (should not be detected)
        let mut content = vec![0x41u8; 8001]; // 'A' repeated
        content[8000] = 0x00; // null byte after first 8000 bytes
        assert!(!is_binary_content(&content));

        // Create content with null byte at position 7999 (should be detected)
        let mut content2 = vec![0x41u8; 8000]; // 'A' repeated
        content2[7999] = 0x00; // null byte within first 8000 bytes
        assert!(is_binary_content(&content2));
    }
}
