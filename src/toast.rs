//! Native Windows Action Center toast notifications for this unpackaged
//! (non-MSIX) app. Toasts from a plain Win32 exe require an App User Model ID
//! (AUMID) that Windows can resolve to a display name - without registering
//! one, `ToastNotificationManager` silently drops the notification. This
//! registers the AUMID under `HKCU\Software\Classes\AppUserModelId\<AUMID>`
//! (the same registry-only technique used by other unpackaged-app toast
//! libraries), which avoids needing a Start Menu shortcut at all.

use windows::core::HSTRING;
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

use crate::native_interop;

const AUMID: &str = "trelowney.ClaudeUsageMonitor";
const AUMID_REGISTRY_PATH: &str = r"Software\Classes\AppUserModelId\trelowney.ClaudeUsageMonitor";
const DISPLAY_NAME: &str = "Claude Usage Monitor (trelowney)";

/// Set the process AUMID and register it with Windows so `notify` below can
/// actually show toasts. Best-effort: failures are logged and otherwise
/// ignored, since notifications are not critical to the app's core function.
pub fn init() {
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(AUMID));
    }

    if let Err(error) = register_aumid() {
        crate::diagnose::log(format!("toast: unable to register AUMID: {error}"));
    }
}

/// Show a toast with the given title/body. Best-effort and silent on
/// failure - a missed notification should never affect the rest of the app.
pub fn notify(title: &str, body: &str) {
    if let Err(error) = show_toast(title, body) {
        crate::diagnose::log(format!("toast: unable to show notification: {error}"));
    }
}

fn show_toast(title: &str, body: &str) -> windows::core::Result<()> {
    let xml = format!(
        "<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>",
        xml_escape(title),
        xml_escape(body)
    );

    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;

    let toast = ToastNotification::CreateToastNotification(&doc)?;
    let notifier = ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(AUMID))?;
    notifier.Show(&toast)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Write `HKCU\Software\Classes\AppUserModelId\<AUMID>\DisplayName`, which is
/// enough for Windows to accept toasts sent under that AUMID and show a
/// sensible name for them in Action Center / notification settings.
fn register_aumid() -> Result<(), String> {
    unsafe {
        let path = native_interop::wide_str(AUMID_REGISTRY_PATH);
        let mut hkey = HKEY::default();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR::from_raw(path.as_ptr()),
            0,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        );
        if result.is_err() {
            return Err(format!("RegCreateKeyExW failed: {result:?}"));
        }

        let value_name = native_interop::wide_str("DisplayName");
        let value_data = native_interop::wide_str(DISPLAY_NAME);
        let data_bytes =
            std::slice::from_raw_parts(value_data.as_ptr() as *const u8, value_data.len() * 2);
        let result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR::from_raw(value_name.as_ptr()),
            0,
            REG_SZ,
            Some(data_bytes),
        );
        let _ = RegCloseKey(hkey);
        if result.is_err() {
            return Err(format!("RegSetValueExW failed: {result:?}"));
        }
        Ok(())
    }
}
