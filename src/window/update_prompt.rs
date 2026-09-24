//! Three-way "new version available" prompt: install now, ask again on the
//! next start, or skip that version for good.
use super::*;
use windows::core::HRESULT;
use windows::Win32::UI::Controls::{
    TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TASKDIALOG_BUTTON,
    TASKDIALOG_FLAGS, TASKDIALOG_NOTIFICATIONS, TDF_ALLOW_DIALOG_CANCELLATION, TDN_CREATED,
    TD_INFORMATION_ICON,
};

const BUTTON_INSTALL: i32 = 100;
const BUTTON_LATER: i32 = 101;
const BUTTON_SKIP: i32 = 102;

/// Only one prompt at a time: a daily re-check must not stack a second dialog
/// on top of one the user has not answered yet.
static PROMPT_OPEN: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UpdateChoice {
    Install,
    /// Ask again on the next app start.
    Later,
    /// Never ask about this version again; a newer one prompts as usual.
    Skip,
}

/// Background checks prompt once per version per run, and never for a
/// version the user chose to skip. A manual check always prompts.
pub(super) fn should_prompt(
    interactive: bool,
    version: &str,
    skipped: Option<&str>,
    deferred: Option<&str>,
) -> bool {
    interactive || (skipped != Some(version) && deferred != Some(version))
}

/// Records the user's answer for a version they did not install.
pub(super) fn remember_choice(state: &mut AppState, choice: UpdateChoice, version: &str) {
    apply_choice(
        &mut state.deferred_update_version,
        &mut state.skipped_update_version,
        choice,
        version,
    );
}

fn apply_choice(
    deferred: &mut Option<String>,
    skipped: &mut Option<String>,
    choice: UpdateChoice,
    version: &str,
) {
    match choice {
        UpdateChoice::Install => {}
        UpdateChoice::Later => *deferred = Some(version.to_string()),
        UpdateChoice::Skip => {
            *skipped = Some(version.to_string());
            *deferred = None;
        }
    }
}

/// Shows the prompt, or returns `None` when another one is already open.
pub(super) fn show_update_prompt(
    strings: Strings,
    release: &ReleaseDescriptor,
) -> Option<UpdateChoice> {
    if PROMPT_OPEN.swap(true, Ordering::AcqRel) {
        return None;
    }
    let choice = run_task_dialog(strings, release).unwrap_or_else(|| {
        // Task dialogs need Common Controls 6 (see app.manifest). If one
        // still fails to open, fall back to the plain two-way question.
        if yes_no_prompt(strings, release) {
            UpdateChoice::Install
        } else {
            UpdateChoice::Later
        }
    });
    PROMPT_OPEN.store(false, Ordering::Release);
    Some(choice)
}

fn prompt_message(strings: Strings, release: &ReleaseDescriptor) -> String {
    strings
        .update_prompt_now
        .replace("{version}", &release.latest_version)
}

fn run_task_dialog(strings: Strings, release: &ReleaseDescriptor) -> Option<UpdateChoice> {
    let title = native_interop::wide_str(strings.update_available);
    let content = native_interop::wide_str(&prompt_message(strings, release));
    let install = native_interop::wide_str(strings.update_prompt_install);
    let later = native_interop::wide_str(strings.update_prompt_later);
    let skip = native_interop::wide_str(strings.update_prompt_skip);
    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: BUTTON_INSTALL,
            pszButtonText: PCWSTR::from_raw(install.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: BUTTON_LATER,
            pszButtonText: PCWSTR::from_raw(later.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: BUTTON_SKIP,
            pszButtonText: PCWSTR::from_raw(skip.as_ptr()),
        },
    ];
    // No owner: the widget is a child of Explorer's taskbar, and a modal
    // dialog owned through it would disable the taskbar while it is open.
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TASKDIALOG_FLAGS(TDF_ALLOW_DIALOG_CANCELLATION.0),
        pszWindowTitle: PCWSTR::from_raw(title.as_ptr()),
        Anonymous1: TASKDIALOGCONFIG_0 {
            pszMainIcon: TD_INFORMATION_ICON,
        },
        pszMainInstruction: PCWSTR::from_raw(title.as_ptr()),
        pszContent: PCWSTR::from_raw(content.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        nDefaultButton: BUTTON_INSTALL,
        pfCallback: Some(bring_to_front),
        ..Default::default()
    };
    let mut pressed = 0;
    unsafe { TaskDialogIndirect(&config, Some(&mut pressed as *mut i32), None, None) }.ok()?;
    Some(choice_for_button(pressed))
}

/// Closing the dialog (Esc, X, Alt+F4) means "not now".
fn choice_for_button(button: i32) -> UpdateChoice {
    match button {
        BUTTON_INSTALL => UpdateChoice::Install,
        BUTTON_SKIP => UpdateChoice::Skip,
        _ => UpdateChoice::Later,
    }
}

/// A prompt raised at login or from a background check would otherwise open
/// behind whatever the user is working in.
unsafe extern "system" fn bring_to_front(
    hwnd: HWND,
    msg: TASKDIALOG_NOTIFICATIONS,
    _wparam: WPARAM,
    _lparam: LPARAM,
    _data: isize,
) -> HRESULT {
    if msg == TDN_CREATED {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE,
        );
        let _ = SetForegroundWindow(hwnd);
    }
    S_OK
}

fn yes_no_prompt(strings: Strings, release: &ReleaseDescriptor) -> bool {
    let title = native_interop::wide_str(strings.update_available);
    let message = native_interop::wide_str(&prompt_message(strings, release));
    unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            MB_YESNO | MB_ICONQUESTION | MB_TOPMOST | MB_SETFOREGROUND,
        ) == IDYES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_checks_respect_skipped_and_deferred_versions() {
        assert!(should_prompt(false, "2.16.0-trelowney.1", None, None));
        assert!(!should_prompt(
            false,
            "2.16.0-trelowney.1",
            Some("2.16.0-trelowney.1"),
            None
        ));
        assert!(!should_prompt(
            false,
            "2.16.0-trelowney.1",
            None,
            Some("2.16.0-trelowney.1")
        ));
        // A newer release than the skipped or deferred one prompts again.
        assert!(should_prompt(
            false,
            "2.16.0-trelowney.2",
            Some("2.16.0-trelowney.1"),
            Some("2.16.0-trelowney.1")
        ));
    }

    #[test]
    fn a_manual_check_always_offers_the_update() {
        assert!(should_prompt(
            true,
            "2.16.0-trelowney.1",
            Some("2.16.0-trelowney.1"),
            Some("2.16.0-trelowney.1")
        ));
    }

    #[test]
    fn a_failed_background_check_is_retried_within_minutes() {
        let checked_at = 1_800_000_000;
        let next_due = retry_soon_timestamp(checked_at) + update_check_interval().as_secs();
        assert_eq!(next_due, checked_at + UPDATE_RETRY_AFTER_ERROR.as_secs());
        assert!(!auto_update_check_due(Some(retry_soon_timestamp(now_unix_secs()))));
    }

    #[test]
    fn closing_the_dialog_means_not_now() {
        assert_eq!(choice_for_button(BUTTON_INSTALL), UpdateChoice::Install);
        assert_eq!(choice_for_button(BUTTON_LATER), UpdateChoice::Later);
        assert_eq!(choice_for_button(BUTTON_SKIP), UpdateChoice::Skip);
        assert_eq!(choice_for_button(IDCANCEL.0), UpdateChoice::Later);
        assert_eq!(choice_for_button(0), UpdateChoice::Later);
    }

    #[test]
    fn skipping_replaces_a_deferral() {
        let mut deferred = None;
        let mut skipped = None;
        for (choice, expected) in [
            (UpdateChoice::Later, (Some("v1"), None)),
            (UpdateChoice::Skip, (None, Some("v1"))),
            (UpdateChoice::Install, (None, Some("v1"))),
        ] {
            apply_choice(&mut deferred, &mut skipped, choice, "v1");
            assert_eq!((deferred.as_deref(), skipped.as_deref()), expected);
        }
    }

    #[test]
    fn the_skipped_version_survives_a_settings_round_trip() {
        let mut settings = SettingsFile::default();
        assert_eq!(settings.skipped_update_version, None);
        settings.skipped_update_version = Some("2.16.0-trelowney.1".into());
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["skipped_update_version"], "2.16.0-trelowney.1");
        let decoded: SettingsFile = serde_json::from_value(json).unwrap();
        assert_eq!(
            decoded.skipped_update_version.as_deref(),
            Some("2.16.0-trelowney.1")
        );
    }
}
