![Windows](https://img.shields.io/badge/platform-Windows-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Claude Usage Monitor (trelowney's build)

This is [CodeZeno](https://codezeno.com.au)'s **Claude Code Usage Monitor** — all credit for the app itself, its design, and everything it does goes to them. See the [original repository](https://github.com/CodeZeno/Claude-Code-Usage-Monitor) for the full feature list, privacy/security details, and how it all works.

This is a personal fork, currently based on their [`v2.2.7`](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/releases/tag/v2.2.7) release, with a handful of personal tweaks on top. Version numbers here follow `<upstream version>-trelowney.<build>`, so it's always clear which upstream release a build is based on — check upstream's [latest release](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/releases/latest) to see if they've moved on since then.

## What's different in this fork

- **The Classic widget docks to the taskbar's left edge** instead of next to the system tray, so it doesn't cover pinned/running app icons on a busy taskbar. Default position leaves a small margin from the very edge so it doesn't land on top of the Windows 11 Widgets/weather button either.
- **No tray icons anywhere** — neither the always-on app icon upstream shows by default, nor the per-model percentage badge icons the Classic theme would otherwise put in the notification area. The widget itself already shows the same numbers. (Theme Studio's own opt-in "show this layer as a tray icon" feature still works if you build a custom theme that asks for it — this fork just doesn't turn it on by default.)
- **Usage bar segments turn red above 80%** for every window (5-hour/7-day, or Cursor's Auto/API, or OpenCode's dynamic weekly/monthly window), for every enabled provider.
- **Toast notifications** (Action Center, bottom-right) for: auth/credential errors, and — for Claude Code, Codex, and Antigravity specifically — a session or weekly window resetting after being above 50%, and a window crossing 80% usage. (Cursor and OpenCode don't get the reset/80% toasts: their "session"/"weekly" fields don't represent a fixed 5-hour/7-day window the same way, so the wording would be misleading.)
- **Auto-updater points at [this repo's releases](https://github.com/trelowney/claude-code-usage-monitor/releases)** instead of upstream's, so it won't ever offer to replace this build with upstream's unmodified one.
- Rebranded binary/window title/menu/Dashboard & Theme Studio window so it's obvious this is the custom build, not upstream.

Everything else — Cursor and OpenCode Go support, the Dashboard, Theme Studio, credentials handling, WSL support, and so on — is unchanged from upstream. Download the latest `claude-usage-monitor-trelowney.exe` from this repo's [Releases](https://github.com/trelowney/claude-code-usage-monitor/releases) page and run it directly; it's portable, no installer.

## Repositioning the widget

Upstream moved from "drag the taskbar widget with the mouse" to a Theme Studio–driven placement model somewhere around `v2.0.0` — the Classic widget's position is now a property of its theme (an `offset_x`/`offset_y` on the theme's placement), not something you drag live on the taskbar. To nudge it:

1. Right-click the widget → **Open Dashboard**.
2. Open the theme editor (Theme Studio) and adjust the Classic theme's placement — you can drag the preview or type an exact offset.

This fork's default already leaves room for the Windows 11 Widgets button, so most people won't need to touch this at all.

## Diagnostics

```powershell
claude-usage-monitor-trelowney.exe --diagnose
```

writes a log to `%TEMP%\claude-code-usage-monitor.log`. Settings live in `%APPDATA%\ClaudeCodeUsageMonitor\settings.json`.

## License

MIT, same as upstream — see [LICENSE](LICENSE).
