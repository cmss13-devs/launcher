#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn get_fixed_runtime_path() -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))?;

    let runtime_path = exe_dir.join("webview2-runtime");
    if runtime_path.exists() {
        return Some(runtime_path);
    }

    None
}

#[cfg(target_os = "windows")]
pub fn setup_fixed_webview2() {
    if let Some(path) = get_fixed_runtime_path() {
        tracing::info!("Using fixed WebView2 runtime at {:?}", path);
        std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", &path);
    }
}

#[cfg(target_os = "windows")]
pub fn check_webview2_installed() -> bool {
    get_fixed_runtime_path().is_some()
}

#[cfg(target_os = "windows")]
pub fn show_webview2_error() {
    use windows::core::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    let config = crate::config::get_config();
    let title = HSTRING::from(format!("{} - Missing Dependency", config.product_name));

    unsafe {
        MessageBoxW(
            None,
            w!("WebView2 Runtime is required but not installed.\n\nPlease reinstall the application."),
            &title,
            MB_OK | MB_ICONERROR,
        );
    }
}
