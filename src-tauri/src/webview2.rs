#[cfg(target_os = "windows")]
pub fn check_webview2_installed() -> bool {
    use winreg::enums::*;
    use winreg::RegKey;

    let paths = [
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
        (
            HKEY_LOCAL_MACHINE,
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
        (
            HKEY_CURRENT_USER,
            r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ),
    ];

    for (hive, path) in paths {
        if let Ok(key) = RegKey::predef(hive).open_subkey(path) {
            if key.get_value::<String, _>("pv").is_ok() {
                return true;
            }
        }
    }
    false
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
            w!("WebView2 Runtime is required but not installed.\n\nPlease download it from:\nhttps://go.microsoft.com/fwlink/p/?LinkId=2124703"),
            &title,
            MB_OK | MB_ICONERROR,
        );
    }
}
