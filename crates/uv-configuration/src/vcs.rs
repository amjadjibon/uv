use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;
use uv_git::GIT;
use uv_jj::JJ;

#[derive(Debug, thiserror::Error)]
pub enum VersionControlError {
    #[error("Attempted to initialize a Git repository, but `git` was not found in PATH")]
    GitNotInstalled,
    #[error("Failed to initialize Git repository at `{0}`\nstdout: {1}\nstderr: {2}")]
    GitInit(PathBuf, String, String),
    #[error("`git` command failed")]
    GitCommand(#[source] std::io::Error),
    #[error("Attempted to initialize a Jujutsu repository, but `jj` was not found in PATH")]
    JjNotInstalled,
    #[error("Failed to initialize Jujutsu repository at `{0}`\nstdout: {1}\nstderr: {2}")]
    JjInit(PathBuf, String, String),
    #[error("`jj` command failed")]
    JjCommand(#[source] std::io::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The version control system to use.
#[derive(Clone, Copy, Debug, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum VersionControlSystem {
    /// Use Git for version control.
    #[default]
    Git,
    /// Use Jujutsu for version control.
    Jj,
    /// Do not use any version control system.
    None,
}

impl VersionControlSystem {
    /// Initializes the VCS system based on the provided path.
    ///
    /// For Jujutsu, if `colocate_with_git` is true, it will initialize in an existing
    /// Git repository using the `--colocate` flag.
    pub fn init(&self, path: &Path, colocate_with_git: bool) -> Result<(), VersionControlError> {
        match self {
            Self::Git => {
                let Ok(git) = GIT.as_ref() else {
                    return Err(VersionControlError::GitNotInstalled);
                };

                let output = Command::new(git)
                    .arg("init")
                    .current_dir(path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .map_err(VersionControlError::GitCommand)?;
                if !output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(VersionControlError::GitInit(
                        path.to_path_buf(),
                        stdout.to_string(),
                        stderr.to_string(),
                    ));
                }

                // Create the `.gitignore`, if it doesn't exist.
                match fs_err::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path.join(".gitignore"))
                {
                    Ok(mut file) => file.write_all(GITIGNORE.as_bytes())?,
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(err) => return Err(err.into()),
                }

                Ok(())
            }
            Self::Jj => {
                let jj = match JJ.as_ref() {
                    Ok(jj) => jj,
                    Err(_) => return Err(VersionControlError::JjNotInstalled),
                };

                let mut command = Command::new(jj);
                command
                    .arg("git")
                    .arg("init")
                    .current_dir(path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                // If colocating with an existing Git repo, use --colocate flag
                if colocate_with_git {
                    command.arg("--colocate");
                }

                let output = command.output().map_err(VersionControlError::JjCommand)?;
                if !output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(VersionControlError::JjInit(
                        path.to_path_buf(),
                        stdout.to_string(),
                        stderr.to_string(),
                    ));
                }

                // Create the `.gitignore`, if it doesn't exist.
                // Jujutsu uses the same .gitignore format as Git
                match fs_err::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path.join(".gitignore"))
                {
                    Ok(mut file) => file.write_all(GITIGNORE.as_bytes())?,
                    Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(err) => return Err(err.into()),
                }

                Ok(())
            }
            Self::None => Ok(()),
        }
    }
}

impl std::fmt::Display for VersionControlSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Git => write!(f, "git"),
            Self::Jj => write!(f, "jj"),
            Self::None => write!(f, "none"),
        }
    }
}

const GITIGNORE: &str = "# Python-generated files
__pycache__/
*.py[oc]
build/
dist/
wheels/
*.egg-info

# Virtual environments
.venv
";

/// Setting for Git LFS (Large File Storage) support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitLfsSetting {
    /// Git LFS is disabled (default).
    #[default]
    Disabled,
    /// Git LFS is enabled. Tracks whether it came from an environment variable.
    Enabled { from_env: bool },
}

impl GitLfsSetting {
    pub fn new(from_arg: Option<bool>, from_env: Option<bool>) -> Self {
        match (from_arg, from_env) {
            (Some(true), _) => Self::Enabled { from_env: false },
            (_, Some(true)) => Self::Enabled { from_env: true },
            _ => Self::Disabled,
        }
    }
}

impl From<GitLfsSetting> for Option<bool> {
    fn from(setting: GitLfsSetting) -> Self {
        match setting {
            GitLfsSetting::Enabled { .. } => Some(true),
            GitLfsSetting::Disabled => None,
        }
    }
}
