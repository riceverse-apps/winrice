# Testing in Windows Sandbox

Everything that changes the system must be tested in a disposable machine, never on your main PC. Windows Sandbox gives a clean Windows 11 that is thrown away when you close it.

## 0. One-time setup on your PC

1. Windows 11 **Pro/Enterprise/Education** and CPU virtualization enabled in the firmware.
2. In an **administrator** PowerShell, then restart:
   ```powershell
   Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -All
   ```
3. Build the app and prepare the kit (normal PowerShell, repo root):
   ```powershell
   npm --prefix app run tauri -- build --no-bundle
   powershell -ExecutionPolicy Bypass -File sandbox\prepare.ps1
   ```
   This copies `winrice.exe` and the scripts to `sandbox\stage` and writes `sandbox\winrice.wsb`. Nothing outside the repo is touched.
4. Double-click `sandbox\winrice.wsb`.

`sandbox\stage` is shared with the Sandbox as `C:\WinRiceTest` (read/write), which is how results come back. Everything else inside the Sandbox disappears when it closes.

## Part A. Verify the tools (resolves the `UNVERIFIED` items)

Inside the Sandbox, wait for the PowerShell window to say *Ready*, then on the desktop run **1-VERIFY-TOOLS**. It installs each of the 20 tools with winget, starts it as its manifest says, records the real paths, process names, new config files, stop/quick-action results, and uninstalls it. It takes a while (20 installs). You can also run one tool: `powershell -File C:\WinRiceTest\verify-tools.ps1 -Only glazewm,zebar`.

When it ends, run **3-COLLECT-RESULTS**. On your PC the files are in `sandbox\stage\results\report\report.txt` and `report.json`. Send them back (or open them yourself, see below).

What the report answers:

| Question | Where in the report |
|---|---|
| Is the winget id right? Did install work? | `install_exit` (0 = ok) |
| Are the declared `detect.paths` right? | `declared_paths[].exists` and `found_exes` (the real location) |
| Is `detect.process` the real process name? | `expected_process_running` and `new_processes` |
| Are `config.paths` right? | `config_paths[].exists` and `new_config_files` |
| Does the stop command work? | `stop.exit`, `stop.process_gone` |
| Does a widget quick action work (GlazeWM `wm-reload-config`)? | `quick_actions[].exit` |
| Does the PowerShell snippet run (Oh My Posh, Starship, Fastfetch)? | `profile_snippet.output` should end with `SNIPPET_OK` |
| Does uninstall remove the files? | `uninstall_exit`, `paths_after_uninstall[].still_exists` |

Fixing a finding = editing the tool's `manifest.yaml`, removing its `# UNVERIFIED` comment, and running `npm run validate` + `cargo test`.

## Part B. Test WinRice itself

Run **2-RUN-WINRICE**. Do the checks in order; note PASS/FAIL and anything odd. After each step you can look at **History** (it shows failures and what was undone).

| # | Check | Expected |
|---|---|---|
| B1 | App opens; onboarding shows 3 screens; switch language and Light/Dark/System | text changes at once; no white flash in dark |
| B2 | Tools: **Install Flow Launcher** (plan dialog → Confirm) | it installs; card says *Installed* |
| B3 | Turn **Flow Launcher** on | process runs; `Alt+Space` opens it; History shows *turned on* |
| B4 | Turn it off | process gone; autostart key removed |
| B5 | Turn on **GlazeWM** with *Try for 30 seconds*, do not confirm | after 30 s it turns off by itself and a message says so |
| B6 | Turn on GlazeWM, press **Keep changes** | stays on |
| B7 | Press `Ctrl+Alt+Shift+R` | the last change is reverted |
| B8 | With GlazeWM on, install **Komorebi**, try to turn it on | blocked with an explanation and a "turn off GlazeWM and continue" button |
| B9 | Press that button | GlazeWM off, Komorebi on, in one step |
| B10 | Install **Oh My Posh**, turn it on, open a new PowerShell | prompt changes; `Documents\PowerShell\...profile.ps1` has a marked block |
| B11 | Turn it off | the block is gone, the rest of the profile is untouched |
| B12 | **Settings tab** of GlazeWM: change *Gap between windows* | `%USERPROFILE%\.glzr\glazewm\config.yaml` is updated and a backup exists in `%APPDATA%\WinRice\backups` |
| B13 | **History**: undo the newest entry | the change is reverted; older entries unlock one by one |
| B14 | **Themes**: open *Catppuccin Dev*, untick Windhawk, **Review and apply** | tools install and turn on |
| B15 | **Home → Undo theme** (close and reopen the app first) | everything the theme did is undone |
| B16 | **Themes → Import** a GitHub repo with a `theme.yaml` (or a bad link) | imported as *unverified*; a bad link gives a clear error |
| B17 | **Save my setup as a theme**, then *Export .zip* | theme under *My themes*; zip in Downloads |
| B18 | Theme page → **Sync colors** (GlazeWM on) | border color changes in GlazeWM's config; undo from History |
| B19 | **Profiles**: save one, turn another tool on, apply the profile | tools match the profile |
| B20 | **Pause customizations**, then **Resume** | everything off, then exactly the same set on again |
| B21 | **Settings → Widget**: enable the tray panel and the floating widget; press `Win+Alt+W` | tray panel opens on click; floating widget toggles; toggles work from both |
| B22 | Widget hardware section | CPU/RAM/disk/network numbers move; *Detailed* shows charts |
| B23 | A real failure: inside the Sandbox disable the network adapter (Settings → Network → Ethernet → Disable), then try to install a tool | the operation fails with a clear message and **nothing is left half done** (History says *failed and undone*) |
| B24 | **Restore original Windows** (with *also uninstall*) | tools off, config files back as they were, WinRice's installs removed |
| B25 | Measure memory: Task Manager → Details, add up `winrice.exe` + `msedgewebview2.exe` with the widget open | write the numbers down (goal in the spec is < 80 MB for the widget) |

## Before closing the Sandbox

Run **3-COLLECT-RESULTS** again. It copies `%APPDATA%\WinRice` (database, backups), the PowerShell profiles, the Run key and the process list to `sandbox\stage\results`. Anything not copied is lost.

## Reporting

For every FAIL: the check number, what you saw, and the matching part of `results\` (especially `WinRice-data\state.db` and `report\report.txt`). A failing step is a bug in the manifest (fix the data) or in the engine (add a test that reproduces it first).
