//! Native Windows Action Center toast notifications for this unpackaged
//! (non-MSIX) app. Toasts from a plain Win32 exe require an App User Model ID
//! (AUMID) that is registered via a Start Menu shortcut carrying that same
//! AUMID as a property - without it, `ToastNotificationManager` silently
//! fails to show anything on most Windows 10/11 builds.

use windows::core::{Interface, HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
use windows::Win32::System::Com::StructuredStorage::{InitPropVariantFromString, PropVariantClear};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, IPersistFile,
};
use windows::Win32::Foundation::TRUE;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PKEY_AppUserModel_ID, PKEY_Title};
use windows::Win32::UI::Shell::{IShellLinkW, SetCurrentProcessExplicitAppUserModelID, ShellLink};

const AUMID: &str = "trelowney.ClaudeUsageMonitor";
const SHORTCUT_NAME: &str = "Claude Usage Monitor (trelowney).lnk";

/// Set the process AUMID and make sure a Start Menu shortcut carrying that
/// AUMID exists, so `notify` below can actually show toasts. Best-effort:
/// failures are logged and otherwise ignored, since notifications are not
/// critical to the app's core function.
pub fn init() {
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(AUMID));
    }

    if let Err(error) = ensure_shortcut() {
        crate::diagnose::log(format!("toast: unable to register shortcut/AUMID: {error}"));
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

fn shortcut_path() -> Result<std::path::PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| format!("no APPDATA: {e}"))?;
    Ok(std::path::PathBuf::from(appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join(SHORTCUT_NAME))
}

fn current_exe_wide() -> Result<Vec<u16>, String> {
    let mut buf = [0u16; 260];
    let len = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    if len == 0 {
        return Err("unable to resolve current executable path".to_string());
    }
    let mut wide: Vec<u16> = buf[..len].to_vec();
    wide.push(0);
    Ok(wide)
}

/// Create (or refresh) the Start Menu shortcut that registers this app's
/// AUMID with the shell, which is required for `ToastNotificationManager` to
/// deliver notifications from an unpackaged executable.
fn ensure_shortcut() -> Result<(), String> {
    let path = shortcut_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create Start Menu dir: {e}"))?;
    }

    let exe_wide = current_exe_wide()?;
    let shortcut_wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let title_wide: Vec<u16> = "Claude Usage Monitor"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let aumid_wide: Vec<u16> = AUMID.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("CoCreateInstance(ShellLink): {e}"))?;

        shell_link
            .SetPath(PCWSTR::from_raw(exe_wide.as_ptr()))
            .map_err(|e| format!("SetPath: {e}"))?;

        let property_store: IPropertyStore = shell_link
            .cast()
            .map_err(|e| format!("cast IPropertyStore: {e}"))?;

        let mut aumid_variant = InitPropVariantFromString(PCWSTR::from_raw(aumid_wide.as_ptr()))
            .map_err(|e| format!("InitPropVariantFromString(aumid): {e}"))?;
        property_store
            .SetValue(&PKEY_AppUserModel_ID, &aumid_variant)
            .map_err(|e| format!("SetValue(AppUserModel_ID): {e}"))?;
        let _ = PropVariantClear(&mut aumid_variant);

        let mut title_variant = InitPropVariantFromString(PCWSTR::from_raw(title_wide.as_ptr()))
            .map_err(|e| format!("InitPropVariantFromString(title): {e}"))?;
        property_store
            .SetValue(&PKEY_Title, &title_variant)
            .map_err(|e| format!("SetValue(Title): {e}"))?;
        let _ = PropVariantClear(&mut title_variant);

        property_store.Commit().map_err(|e| format!("Commit: {e}"))?;

        let persist_file: IPersistFile = shell_link
            .cast()
            .map_err(|e| format!("cast IPersistFile: {e}"))?;
        persist_file
            .Save(PCWSTR::from_raw(shortcut_wide.as_ptr()), TRUE)
            .map_err(|e| format!("Save: {e}"))?;
    }

    Ok(())
}
