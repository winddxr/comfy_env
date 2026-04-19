use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

pub struct TempWorkdir {
    path: PathBuf,
}

impl TempWorkdir {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn read_text_if_exists(path: &Path) -> std::io::Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    fs::read_to_string(path).map(Some)
}

pub fn atomic_write(target: &Path, content: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .with_context(|| format!("{} has no parent directory", target.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;

    let mut temp = NamedTempFile::new_in(parent)
        .with_context(|| format!("failed to create temp file in {}", parent.display()))?;
    temp.write_all(content)
        .with_context(|| format!("failed to write temp file for {}", target.display()))?;
    temp.as_file_mut()
        .sync_all()
        .with_context(|| format!("failed to flush temp file for {}", target.display()))?;

    temp.persist(target)
        .with_context(|| format!("failed to replace {}", target.display()))?;

    sync_parent_dir(parent);
    Ok(())
}

pub fn sha256(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_if_exists(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    sha256(path).map(Some)
}

pub fn create_workdir(base: &Path) -> Result<TempWorkdir> {
    fs::create_dir_all(base).with_context(|| format!("failed to create {}", base.display()))?;
    let temp = tempfile::Builder::new()
        .prefix("gov-")
        .tempdir_in(base)
        .with_context(|| format!("failed to create workdir in {}", base.display()))?;
    let path = temp.keep();
    Ok(TempWorkdir { path })
}

pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let parent = dst
        .parent()
        .with_context(|| format!("{} has no parent directory", dst.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;
    fs::copy(src, dst)
        .with_context(|| format!("failed to copy {} to {}", src.display(), dst.display()))?;
    Ok(())
}

pub fn backup_with_marker(src: &Path, backup: &Path, marker: &Path) -> Result<()> {
    if src.exists() {
        copy_file(src, backup)?;
        atomic_write(marker, b"1")?;
    } else {
        atomic_write(marker, b"0")?;
    }
    Ok(())
}

pub fn restore_with_marker(backup: &Path, dst: &Path, marker: &Path) -> Result<()> {
    let marker_value = fs::read_to_string(marker)
        .with_context(|| format!("failed to read {}", marker.display()))?;

    match marker_value.trim() {
        "1" => {
            if !backup.exists() {
                bail!("backup file is missing: {}", backup.display());
            }
            let bytes = fs::read(backup)
                .with_context(|| format!("failed to read {}", backup.display()))?;
            atomic_write(dst, &bytes)?;
        }
        "0" => {
            if dst.exists() {
                remove_file_if_exists(dst)?;
            }
        }
        other => bail!("invalid existence marker `{other}` in {}", marker.display()),
    }

    Ok(())
}

pub fn remove_file_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    clear_readonly_if_needed(path)?;
    fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    Ok(())
}

fn clear_readonly_if_needed(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
    let mut permissions = metadata.permissions();
    if permissions.readonly() {
        #[cfg(windows)]
        {
            permissions.set_readonly(false);
            fs::set_permissions(path, permissions)
                .with_context(|| format!("failed to update permissions for {}", path.display()))?;
        }
    }
    Ok(())
}

fn sync_parent_dir(parent: &Path) {
    #[cfg(windows)]
    {
        let _ = OpenOptions::new().read(true).open(parent).and_then(|file| file.sync_all());
    }

    #[cfg(not(windows))]
    {
        let _ = OpenOptions::new().read(true).open(parent).and_then(|file| file.sync_all());
    }
}
