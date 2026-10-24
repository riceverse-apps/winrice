# What has been verified on a real Windows 11

Run on 2026-10-24 in a clean **Windows Sandbox** (Windows 11 24H2, build 26100) with `sandbox/prepare.ps1 -Auto`. For each tool the script ran the exact `winget install` of the manifest, checked the declared paths, started the tool the way the manifest says, waited for the process, ran the stop command and the widget quick actions, read the config the tool generated, then uninstalled it. See [vm-testing.md](vm-testing.md) to repeat it.

| Tool | Install | Declared paths | Start | Stop / quick actions | Notes |
|---|---|---|---|---|---|
| GlazeWM | ok | ok (both programs) | ok (`start`) | `wm-exit` ok, `wm-reload-config` ok | generated config confirms `gaps.inner_gap: '20px'` and `window_effects.focused_window.border.color`. Its default config also starts and closes Zebar |
| Zebar | ok (already installed through GlazeWM) | ok | ok | process killed | |
| Komorebi | ok | ok | `komorebic start` ok | `komorebic stop` ok | `komorebi.json` is not created on first start, so the config path is still a convention |
| Flow Launcher | ok | ok | ok | | |
| YASB | ok | ok | ok | | |
| Rainmeter | ok | ok | ok | | |
| Lively | ok | ok (`%ProgramFiles%`) | ok | | |
| PowerToys | ok | ok (`%LOCALAPPDATA%`, per-user) | ok | | |
| Windhawk | ok | ok | `-tray-only` ok | | |
| Fastfetch | ok | ok (WinGet Links) | | | profile snippet runs |
| Starship | ok | ok | | | profile snippet runs |
| Oh My Posh | ok (MSIX `ohmyposh.cli`) | ok (execution alias in `WindowsApps`) | | | profile snippet runs |
| Windows Terminal | ok | ok (`wt.exe` alias) | | | |
| Nerd Font | ok | ok (`%WINDIR%\Fonts`) | | | |
| Nilesoft Shell, ExplorerPatcher, StartAllBack, Start11 | ok | ok | | | install/uninstall only, no toggle |
| Seelen UI | ok (MSIX) | ok (`slu`, `slu-service` aliases) | works on a real PC; did **not** start inside the Sandbox | `slu-service stop` works on a real PC | its own scheduled task restarts it at sign-in (handled by `autostart_task`) |

## Not confirmed

- **TranslucentTB**: `winget install` failed in the Sandbox with `0x80073CF3` (a framework dependency of the MSIX package was missing there). Retry on a real PC; its application id is still a guess.
- **Uninstall** of ExplorerPatcher and Start11 timed out and GlazeWM's uninstall failed while its processes were still alive; programs started by a test keep their files locked. This is a limit of the test script, not of the manifests.
- **WinRice itself** (toggles, try mode, themes, undo, widget, restore) has **not** been exercised in the Sandbox yet: the checklist B1-B25 in [vm-testing.md](vm-testing.md) is still to be done.
- Wallpaper change (`UpdatePerUserSystemParameters`) and the administrator prompt for scheduled tasks need a real desktop session.
