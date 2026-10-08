use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn validate_build_paths(root: &Path, out: &Path) -> Result<(PathBuf, PathBuf)> {
    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize root directory '{}'", root.display()))?;

    // Check root contains Cargo.toml and content
    if !canonical_root.join("Cargo.toml").exists() {
        bail!(
            "Root directory '{}' must contain Cargo.toml",
            root.display()
        );
    }
    if !canonical_root.join("content").exists() {
        bail!(
            "Root directory '{}' must contain 'content' directory",
            root.display()
        );
    }

    let out_path = if out.is_absolute() {
        out.to_path_buf()
    } else {
        canonical_root.join(out)
    };

    if out_path.is_symlink() {
        bail!("Output directory '{}' cannot be a symlink", out.display());
    }

    // Resolve path for containment checks even if out_path does not exist yet
    let normalized_out = if let Ok(canon) = out_path.canonicalize() {
        canon
    } else {
        let mut cur = out_path.clone();
        let mut tails = Vec::new();
        while !cur.exists() {
            if let Some(parent) = cur.parent() {
                if let Some(file_name) = cur.file_name() {
                    tails.push(file_name.to_os_string());
                }
                cur = parent.to_path_buf();
            } else {
                break;
            }
        }
        if let Ok(canon_parent) = cur.canonicalize() {
            let mut resolved = canon_parent;
            for part in tails.into_iter().rev() {
                resolved.push(part);
            }
            resolved
        } else {
            out_path.clone()
        }
    };

    if normalized_out == canonical_root {
        bail!("Output directory cannot be identical to root directory");
    }
    if normalized_out.starts_with(canonical_root.join("content")) {
        bail!("Output directory cannot be inside content directory");
    }
    if normalized_out.starts_with(canonical_root.join("crates")) {
        bail!("Output directory cannot be inside crates directory");
    }
    if normalized_out.starts_with(canonical_root.join(".git")) {
        bail!("Output directory cannot be inside .git directory");
    }

    Ok((canonical_root, out_path))
}

pub struct StagingDirectory {
    pub staging_path: PathBuf,
    pub target_path: PathBuf,
    committed: bool,
}

impl StagingDirectory {
    pub fn new(target: &Path) -> Result<Self> {
        let parent = target.parent().unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let name = target.file_name().unwrap_or_default().to_string_lossy();
        let staging_path = parent.join(format!(".{}.staging", name));
        if staging_path.exists() {
            fs::remove_dir_all(&staging_path)?;
        }
        fs::create_dir_all(&staging_path)?;
        Ok(Self {
            staging_path,
            target_path: target.to_path_buf(),
            committed: false,
        })
    }

    pub fn commit(mut self) -> Result<()> {
        let backup_path = self.target_path.with_extension("backup");
        let target_existed = self.target_path.exists();

        if target_existed {
            if backup_path.exists() {
                fs::remove_dir_all(&backup_path)?;
            }
            fs::rename(&self.target_path, &backup_path)?;
        }

        if let Err(err) = fs::rename(&self.staging_path, &self.target_path) {
            // Rollback
            if target_existed && backup_path.exists() {
                let _ = fs::rename(&backup_path, &self.target_path);
            }
            return Err(err).context("Failed to promote staging directory to target output");
        }

        self.committed = true;
        if target_existed && backup_path.exists() {
            let _ = fs::remove_dir_all(&backup_path);
        }

        Ok(())
    }
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        if !self.committed && self.staging_path.exists() {
            let _ = fs::remove_dir_all(&self.staging_path);
        }
    }
}
