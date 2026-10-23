# Theme reference

File: `theme.yaml`. Schema: `schemas/theme.schema.json`; model: `app/src-tauri/src/theme.rs`.

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | kebab-case, equal to the folder name |
| `name` | yes | display name |
| `author` | yes | `name` and optionally `github` |
| `version` | yes | `MAJOR.MINOR.PATCH` |
| `license` | yes | license of the theme (the official library needs an open one) |
| `description` | yes | *i18n*: `en` and `pt-BR` |
| `tags`, `difficulty` | no | used by the gallery filters |
| `requires.windows_min_build` | no | the theme is shown as incompatible below it |
| `palette` | no | `name` + `colors` (`#rrggbb`). Role names (`accent`, `base`, `text`, ...) are matched by *Sync colors* |
| `components` | yes | at least one (below) |
| `wallpaper` | no | image inside the theme, set as the desktop wallpaper |
| `media` | no | `cover`, `screenshots`, `demo` |

## `components`

| Field | Meaning |
|---|---|
| `tool` | id of a tool in the catalog |
| `config` | file or folder inside the theme. A file replaces the tool's main config; a folder is copied next to it |
| `settings` | `{ key: value }` for options declared in the tool manifest |
| `font` | informational (the font tool is installed as a component) |
| `mods` | Windhawk mods; **listed but not applied yet** |
| `required` | unticking a required component shows a warning |

## How a theme is applied

For each selected component, in order: install the tool if missing → write its config files → write its `settings` → turn it on (tools that clash and are on are turned off first) → finally the wallpaper. One transaction: any failure reverts everything. The result can be undone from the Home page or the History, even after restarting the app.

## What is rejected

Executable or script file types, paths that leave the theme folder, bundles over 8 MB, invalid colors, unknown tools, unknown `settings` keys, components that cannot run together, binary config files.
