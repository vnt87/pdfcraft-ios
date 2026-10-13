//! Assemble native services without exposing platform objects to the shared UI.
use std::path::PathBuf;

/// The app's support folder, where its settings, recovery files and managed documents live.
pub struct Platform {
    pub root: PathBuf,
}

fn support_directory() -> Result<PathBuf, String> {
    use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};
    let directories = NSFileManager::defaultManager()
        .URLsForDirectory_inDomains(NSSearchPathDirectory::ApplicationSupportDirectory, NSSearchPathDomainMask::UserDomainMask);
    let path = directories.firstObject().and_then(|url| url.path()).ok_or("iOS Application Support directory is unavailable")?;
    let expected = crate::storage::app_support_dir(std::env::var_os("HOME"))?;
    let root = PathBuf::from(path.to_string()).join(expected.file_name().ok_or("Application Support name is missing")?);
    // The OS can spell the same container through /var or /private/var.
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    if root.canonicalize().map_err(|error| error.to_string())? != expected.canonicalize().map_err(|error| error.to_string())? {
        return Err("Application Support is outside the expected app container".into());
    }
    Ok(root)
}

/// Find the support folder and install the document pickers (`rfd::AsyncFileDialog` shows UIKit's).
pub fn create() -> Result<Platform, String> {
    let root = support_directory()?;
    crate::documents::install(root.clone())?;
    Ok(Platform { root })
}
