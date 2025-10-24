use std::process::Command;
use std::path::Path;

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
    
    // Try to run jj status to see if we're in a jj repo without .jj directory
    let jj_status = Command::new("jj")
        .arg("status")
        .output();
    
    if let Ok(output) = jj_status {
        if output.status.success() {
            return Ok(VcsType::Jujutsu);
        }
    }
    
    // Try to run git status to see if we're in a git repo
    let git_status = Command::new("git")
        .arg("status")
        .output();
    
    if let Ok(output) = git_status {
        if output.status.success() {
            return Ok(VcsType::Git);
        }
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
    let mut cmd = Command::new("jj");
    cmd.arg("diff");
    
    if let Some(rev) = revision {
        cmd.arg("-r").arg(rev);
    }
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get jj diff: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Gets the current description for a Jujutsu revision
pub fn get_jj_description(revision: Option<&str>) -> anyhow::Result<Option<String>> {
    let mut cmd = Command::new("jj");
    cmd.arg("log")
        .arg("-r")
        .arg(revision.unwrap_or("@"))
        .arg("--no-graph")
        .arg("--template")
        .arg("{description}");
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get jj description: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let description = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    if description.is_empty() {
        Ok(None)
    } else {
        Ok(Some(description))
    }
}

/// Sets the description for a Jujutsu revision
pub fn set_jj_description(revision: Option<&str>, description: &str) -> anyhow::Result<()> {
    let mut cmd = Command::new("jj");
    cmd.arg("describe");
    
    if let Some(rev) = revision {
        cmd.arg("-r").arg(rev);
    }
    
    cmd.arg("-m").arg(description);
    
    let output = cmd.output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to set jj description: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    Ok(())
}

/// Checks if there are any changes in the working directory for Jujutsu
pub fn has_jj_changes() -> anyhow::Result<bool> {
    let output = Command::new("jj")
        .arg("status")
        .arg("--porcelain")
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to check jj status: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let status = String::from_utf8_lossy(&output.stdout);
    Ok(!status.trim().is_empty())
}

/// Gets the list of modified files for Jujutsu
pub fn get_jj_modified_files() -> anyhow::Result<Vec<String>> {
    let output = Command::new("jj")
        .arg("status")
        .arg("--porcelain")
        .output()?;
    
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "Failed to get jj status: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    
    let status = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = status
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .collect();
    
    Ok(files)
}