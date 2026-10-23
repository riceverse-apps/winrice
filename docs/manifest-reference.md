# Tool manifest reference

File: `catalog/tools/<id>/manifest.yaml`. The JSON Schema is `schemas/tool.schema.json`; the Rust model is `app/src-tauri/src/manifest.rs`. Text fields marked *i18n* need `en` and `pt-BR`.

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | kebab-case, equal to the folder name |
| `name` | yes | display name |
| `category` | yes | `window-manager`, `status-bar`, `taskbar-start`, `shell-mod`, `terminal`, `launcher`, `wallpaper`, `widgets`, `icons-cursors`, `file-manager` |
| `summary` | yes | one line (*i18n*) |
| `difficulty` | yes | `beginner`, `intermediate`, `advanced` |
| `homepage`, `license` | yes | official page and SPDX-like license |
| `paid`, `restricted_license` | no | flags shown in the UI |
| `requires.windows_min_build` | yes | e.g. `22000` for Windows 11 |
| `requires.windows_max_build` | no | set when a build breaks the tool; outside the range the tool is *incompatible* |
| `requires.admin` | no | installing it asks for administrator permission |
| `install` | yes | `method: winget` + `id` (other methods are declared in the schema but not implemented yet) |
| `uninstall` | no | `method: winget` |
| `detect.process` | no | process name; used to know if the tool is running and to verify it started |
| `detect.paths` | yes | files whose existence means "installed"; also used to find the real path of programs |
| `activate.start` | no | command to start it (the program is resolved through `detect.paths`) |
| `activate.autostart` | no | register it to start with Windows (per-user Run key `WinRice-<id>`) |
| `activate.profile_snippet` | no | `files` + `content`: a marked block added to the PowerShell profile |
| `deactivate.stop` | no | command to stop it; if absent the process from `detect.process` is ended |
| `deactivate.remove_autostart` | no | remove the autostart entry |
| `config.paths` | no | config files; the first is the main one |
| `config.format` | no | `yaml`, `json` (needed for `settings`), others are informational |
| `config.backup` | no | back the files up before activating |
| `conflicts` | no | ids that cannot be on at the same time (declare on both sides) |
| `pairs_well_with` | no | ids shown as suggestions |
| `changes_on_screen` | no | list of *i18n* sentences: what visibly changes |
| `shortcuts` | no | `keys` + `en` + `pt-BR` |
| `risks`, `how_to_undo` | no | *i18n* |
| `settings` | no | simple options (below) |
| `widget` | no | `show_by_default`, `quick_actions: [{id, label (i18n), command}]` |

A tool **without** `activate`/`deactivate` has no toggle (install/uninstall only).

Paths may use environment variables such as `%ProgramFiles%`, `%LOCALAPPDATA%`, `%APPDATA%`, `%USERPROFILE%`.

## `settings`

```yaml
settings:
  - key: gaps.inner_gap      # dotted path inside the config file
    type: number             # number | boolean | string | color
    suffix: px               # number only: written as "12px", read back as 12
    min: 0
    max: 200
    default: 20
    palette: accent          # color only: palette role written by "Sync colors"
    label: { en: "Gap between windows", pt-BR: "Espaço entre janelas" }
```

Values are validated (type and range) before being written. Writing YAML does not keep comments; the original file is always backed up first.

## Validation (CI)

`npm run validate` checks the schema, that the folder name equals the `id`, that both docs files exist, that every `conflicts` id exists and, for the whole catalog, that locales have the same keys in both languages.
