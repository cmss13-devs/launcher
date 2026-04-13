//! URL opening utilities that work in `AppImage` environments.

use crate::error::{CommandError, CommandResult};

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
const SYSTEM_PATHS: &[&str] = &["/usr/bin", "/usr/local/bin", "/bin"];

#[cfg(target_os = "linux")]
fn is_system_path(path: &str) -> bool {
    let is_system =
        path.starts_with("/usr/") || path.starts_with("/bin/") || path.starts_with("/sbin/");
    let product_name = crate::config::get_config().product_name;
    let is_bundled = path.contains(product_name);
    is_system && !is_bundled
}

#[cfg(target_os = "linux")]
fn find_xdg_open() -> Option<String> {
    // Check if user has explicitly set BROWSER to a real system path
    if let Ok(browser) = std::env::var("BROWSER") {
        if !browser.is_empty() && is_system_path(&browser) {
            return Some(browser);
        }
    }

    // Check standard system paths FIRST
    for dir in SYSTEM_PATHS {
        let path = format!("{}/xdg-open", dir);
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    // Fall back to which, but filter out bundled paths
    if let Ok(output) = Command::new("which").arg("xdg-open").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && is_system_path(&path) {
                return Some(path);
            }
        }
    }

    None
}

/// Open a URL in the default browser.
/// On Linux, tries system xdg-open first to work around AppImage bundling issues.
#[cfg(target_os = "linux")]
pub fn open(url: &str) -> CommandResult<()> {
    // Try system xdg-open first
    if let Some(xdg_open) = find_xdg_open() {
        let mut cmd = Command::new(&xdg_open);
        cmd.arg(url);
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        if let Ok(mut child) = cmd.spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            return Ok(());
        }
    }

    // Fall back to open crate
    open::that(url).map_err(CommandError::from)
}

#[cfg(not(target_os = "linux"))]
pub fn open(url: &str) -> CommandResult<()> {
    open::that(url).map_err(CommandError::from)
}
