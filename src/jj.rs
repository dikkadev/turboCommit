use std::path::Path;

use jj_lib::config::StackedConfig;
use jj_lib::matchers::EverythingMatcher;
use jj_lib::repo::Repo as _;
use jj_lib::settings::UserSettings;
use jj_lib::workspace::Workspace;
use jj_lib::backend::TreeValue;
use futures::StreamExt;
use pollster::FutureExt;
use tokio::io::AsyncReadExt;
use git2;

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
    
    if let Ok(_workspace) = Workspace::load(&user_settings, Path::new("."), &store_factories, &working_copy_factories) {
        return Ok(VcsType::Jujutsu);
    }
    
    // Try to discover git repository using git2
    if git2::Repository::discover(".").is_ok() {
        return Ok(VcsType::Git);
    }
    
    Err(anyhow::anyhow!("No supported VCS repository found. Please run this command from within a git or jj repository."))
}

/// Validates that a revision string is a simple ID (not a complex expression)
pub fn validate_revision_id(rev: &str) -> anyhow::Result<()> {
    // Simple validation: should be alphanumeric with possible hyphens
    // This prevents complex expressions like ranges, functions, etc.
    if rev.is_empty() {
        return Err(anyhow::anyhow!("Revision cannot be empty"));
    }
    
    // Allow alphanumeric characters, hyphens, and dots (for commit hashes)
    if !rev.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '.') {
        return Err(anyhow::anyhow!(
            "Revision must be a simple ID (alphanumeric, hyphens, dots only). Complex expressions are not supported."
        ));
    }
    
    Ok(())
}

/// Gets the diff for Jujutsu VCS
pub fn get_jj_diff(revision: Option<&str>) -> anyhow::Result<String> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();
    
    let workspace = Workspace::load(&user_settings, Path::new("."), &store_factories, &working_copy_factories)?;
    let repo = workspace.repo_loader().load_at_head()?;
    
    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo.view().get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Parse revision as commit ID
        let commit_id = jj_lib::backend::CommitId::try_from_hex(rev)
            .ok_or_else(|| anyhow::anyhow!("Invalid commit ID: {}", rev))?;
        repo.store().get_commit(&commit_id)?
    };
    
    let tree = commit.tree()?;
    let parent_tree = commit.parent_tree(repo.as_ref())?;
    
    
    // Generate proper diff using jj-lib API
    // tree and parent_tree are already MergedTree instances
    let merged_tree = tree;
    let merged_parent_tree = parent_tree;
    let matcher = EverythingMatcher;
    
    let diff_stream = merged_tree.diff_stream(&merged_parent_tree, &matcher);
    let mut diff_result = String::new();
    
    // Collect all diff entries and iterate through them
    let entries: Vec<jj_lib::merged_tree::TreeDiffEntry> = diff_stream.collect::<Vec<_>>().block_on();
    for entry in entries {
        let path = &entry.path;
        let path_str = path.as_internal_file_string();
        
        // Get source (before) and target (after) values
        let diff = entry.values.as_ref().map_err(|e| anyhow::anyhow!("Diff error: {}", e))?;
        let source_value = &diff.before;
        let target_value = &diff.after;
        
        // Determine change type and generate appropriate diff
        match (source_value.as_resolved(), target_value.as_resolved()) {
            // File deleted
            (Some(Some(TreeValue::File { id: source_id, .. })), None) => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("deleted file mode 100644\n"));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ /dev/null\n"));
                
                let content = read_file_content(repo.store(), path, &source_id).block_on()?;
                diff_result.push_str(&format_deletion(&content));
            }
            
            // File added
            (None, Some(Some(TreeValue::File { id: target_id, .. }))) => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("new file mode 100644\n"));
                diff_result.push_str(&format!("--- /dev/null\n"));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                
                let content = read_file_content(repo.store(), path, &target_id).block_on()?;
                diff_result.push_str(&format_addition(&content));
            }
            
            // File modified
            (
                Some(Some(TreeValue::File { id: source_id, executable: source_exec, .. })),
                Some(Some(TreeValue::File { id: target_id, executable: target_exec, .. }))
            ) if source_id != target_id || source_exec != target_exec => {
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
                
                let source_content = read_file_content(repo.store(), path, &source_id).block_on()?;
                let target_content = read_file_content(repo.store(), path, &target_id).block_on()?;
                
                diff_result.push_str(&format_unified_diff(
                    &source_content,
                    &target_content,
                    3, // context lines
                )?);
            }
            
            // Symlink changes
            (Some(Some(TreeValue::Symlink(source_id))), Some(Some(TreeValue::Symlink(target_id))))
                if source_id != target_id => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str("@@ -1 +1 @@\n");
                
                let source_target = read_symlink(repo.store(), path, &source_id).block_on()?;
                let target_target = read_symlink(repo.store(), path, &target_id).block_on()?;
                diff_result.push_str(&format!("-{}\n", source_target));
                diff_result.push_str(&format!("+{}\n", target_target));
            }
            
            // File type changes (e.g., file to symlink)
            (Some(Some(source)), Some(Some(target))) if std::mem::discriminant(source) != std::mem::discriminant(target) => {
                diff_result.push_str(&format!("diff --git a/{} b/{}\n", path_str, path_str));
                diff_result.push_str(&format!("--- a/{}\n", path_str));
                diff_result.push_str(&format!("+++ b/{}\n", path_str));
                diff_result.push_str(&format!("File type changed\n"));
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
fn format_unified_diff(
    source: &[u8],
    target: &[u8],
    _context_lines: usize,
) -> anyhow::Result<String> {
    let source_text = String::from_utf8_lossy(source);
    let target_text = String::from_utf8_lossy(target);
    
    let source_lines: Vec<&str> = source_text.lines().collect();
    let target_lines: Vec<&str> = target_text.lines().collect();
    
    let mut output = String::new();
    
    // Simple line-by-line diff for now
    let max_lines = source_lines.len().max(target_lines.len());
    
    for i in 0..max_lines {
        let source_line = source_lines.get(i).unwrap_or(&"");
        let target_line = target_lines.get(i).unwrap_or(&"");
        
        if source_line != target_line {
            if !output.contains("@@") {
                output.push_str(&format!("@@ -{},{} +{},{} @@\n", 
                    i + 1, source_lines.len(), i + 1, target_lines.len()));
            }
            
            if !source_line.is_empty() {
                output.push_str(&format!("-{}\n", source_line));
            }
            if !target_line.is_empty() {
                output.push_str(&format!("+{}\n", target_line));
            }
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
    
    let workspace = Workspace::load(&user_settings, Path::new("."), &store_factories, &working_copy_factories)?;
    let repo = workspace.repo_loader().load_at_head()?;
    
    // Resolve revision (default to @)
    let rev = revision.unwrap_or("@");
    let commit = if rev == "@" {
        // Get the working copy commit
        let wc_commit_id = repo.view().get_wc_commit_id(workspace.workspace_name())
            .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
        repo.store().get_commit(wc_commit_id)?
    } else {
        // Parse revision as commit ID
        let commit_id = jj_lib::backend::CommitId::try_from_hex(rev)
            .ok_or_else(|| anyhow::anyhow!("Invalid commit ID: {}", rev))?;
        repo.store().get_commit(&commit_id)?
    };
    
    let description = commit.description();
    
    
    if description.is_empty() {
        Ok(None)
    } else {
        Ok(Some(description.to_string()))
    }
}

/// Sets the description for a Jujutsu revision
pub fn set_jj_description(_revision: Option<&str>, _description: &str) -> anyhow::Result<()> {
    // For now, just return an error indicating this is not implemented
    // TODO: Implement proper commit description setting
    Err(anyhow::anyhow!("Setting jj descriptions is not yet implemented with jj-lib"))
}

/// Checks if there are any changes in the working directory for Jujutsu
pub fn has_jj_changes() -> anyhow::Result<bool> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();
    
    let workspace = Workspace::load(&user_settings, Path::new("."), &store_factories, &working_copy_factories)?;
    let repo = workspace.repo_loader().load_at_head()?;
    
    // Get the working copy commit
    let wc_commit_id = repo.view().get_wc_commit_id(workspace.workspace_name())
        .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
    let wc_commit = repo.store().get_commit(wc_commit_id)?;
    
    // Get the parent tree
    let _parent_tree = wc_commit.parent_tree(repo.as_ref())?;
    let _current_tree = wc_commit.tree()?;
    
    // Check if trees are different
    Ok(_parent_tree.id() != _current_tree.id())
}

/// Gets the list of modified files for Jujutsu
pub fn get_jj_modified_files() -> anyhow::Result<Vec<String>> {
    let config = StackedConfig::with_defaults();
    let user_settings = UserSettings::from_config(config)?;
    let store_factories = jj_lib::repo::StoreFactories::default();
    let working_copy_factories = jj_lib::workspace::default_working_copy_factories();
    
    let workspace = Workspace::load(&user_settings, Path::new("."), &store_factories, &working_copy_factories)?;
    let repo = workspace.repo_loader().load_at_head()?;
    
    // Get the working copy commit
    let wc_commit_id = repo.view().get_wc_commit_id(workspace.workspace_name())
        .ok_or_else(|| anyhow::anyhow!("No working copy commit found"))?;
    let wc_commit = repo.store().get_commit(wc_commit_id)?;
    
    // Get the parent tree
    let _parent_tree = wc_commit.parent_tree(repo.as_ref())?;
    let _current_tree = wc_commit.tree()?;
    
    // For now, just return empty list
    // TODO: Implement proper file listing from tree diff
    Ok(Vec::new())
}