![Windows](https://img.shields.io/badge/platform-Windows-blue)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Claude Usage Monitor (trelowney's build)

This is [CodeZeno](https://codezeno.com.au)'s **Claude Code Usage Monitor** — all credit for the app itself, its design, and everything it does goes to them. See the [original repository](https://github.com/CodeZeno/Claude-Code-Usage-Monitor) for the full feature list, privacy/security details, and how it all works.

This is a private fork, forked from their [`v1.4.9`](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/releases/tag/v1.4.9) release (commit `7b108da`), with a handful of personal tweaks on top. Version numbers here follow `<upstream version>-trelowney.<build>`, so it's always clear which upstream release a build is based on — check upstream's [latest release](https://github.com/CodeZeno/Claude-Code-Usage-Monitor/releases/latest) to see if they've moved on since then.

## What's different in this fork

- **Widget docks to the left edge of the taskbar** instead of next to the system tray, so it doesn't cover pinned/running app icons on a busy taskbar.
- **No system tray icons** — the percentage badges near the clock are gone; the widget itself already shows the same numbers.
- **Toast notifications** for auth/token expiry (Action Center, bottom-right) instead of a tray balloon, since there's no tray icon left to hang a balloon off of.
- **Auto-updater points at [this repo's releases](https://github.com/trelowney/claude-code-usage-monitor/releases)** instead of upstream's, so it won't ever offer to replace this build with upstream's unmodified one.
- Rebranded binary/window title/menu so it's obvious this is the custom build, not upstream.

Everything else — reading usage, the bars, the models menu, credentials handling, WSL support, and so on — is unchanged from upstream. Download the latest `claude-usage-monitor-trelowney.exe` from this repo's [Releases](https://github.com/trelowney/claude-code-usage-monitor/releases) page and run it directly; it's portable, no installer.

## One thing to know: left-edge overlap

Because the widget docks flush against the taskbar's left edge, it can end up sitting on top of another element that also lives there — most commonly the Windows 11 **Widgets/weather board** button. If that happens:

1. Hover the very left edge of the widget (a thin strip — the cursor turns into a left/right resize arrow).
2. Click and drag it to the right until it clears whatever it was covering.
3. Let go — the position is saved automatically and survives restarts.

## Diagnostics

```powershell
claude-usage-monitor-trelowney.exe --diagnose
```

writes a log to `%TEMP%\claude-code-usage-monitor.log`. Settings live in `%APPDATA%\ClaudeCodeUsageMonitor\settings.json`.

## License

MIT, same as upstream — see [LICENSE](LICENSE).
