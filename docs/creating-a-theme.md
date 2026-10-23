# Creating a theme

A theme is a folder with a `theme.yaml` plus the files it refers to. Official themes live in `catalog/themes/<theme-id>/`; yours live in `%APPDATA%\WinRice\themes\`.

## The easy way: from the app

1. Turn on the tools you want and adjust them.
2. **Themes → Save my current setup as a theme.** WinRice collects the config files of the tools that are on (PowerShell profiles are never copied) and writes the theme.
3. To share it: open the theme under *My themes* → **Send to the official library**. The wizard checks the rules, lets you fix the details, adds a cover screenshot and prepares the folder and the pull request text.
4. **Export .zip** gives you the theme as a file to share anywhere.

## By hand

```yaml
id: my-theme                 # kebab-case, same as the folder name
name: My Theme
author: { name: You, github: your-user }
version: 1.0.0
license: MIT
description:
  en: "What it looks like."
  pt-BR: "Como ele fica."
tags: [dev, dark]
difficulty: beginner         # beginner | intermediate | advanced
requires: { windows_min_build: 22621 }
palette:
  name: my-palette
  colors: { base: "#1e1e2e", text: "#cdd6f4", accent: "#cba6f7" }
components:
  - tool: glazewm
    config: configs/glazewm/config.yaml   # file or folder inside the theme
    settings: { gaps.inner_gap: 12 }      # options declared in the tool manifest
    required: true
  - tool: zebar
    required: false
wallpaper: assets/wallpaper.png
media: { cover: screenshots/cover.png, screenshots: [screenshots/cover.png] }
```

See the [theme reference](theme-reference.md) for every field.

## Rules

- **Data only.** Allowed: `yaml yml json jsonc toml css ini md txt` and images (`png jpg jpeg webp gif bmp`). Anything else is rejected, by the app and by CI.
- Paths are relative to the theme folder; no `..`, no absolute paths. Maximum size 8 MB.
- Every `tool` must exist in `catalog/tools/`, and two tools that conflict cannot be in the same theme.
- A single-file `config` replaces the tool's main config file; a **folder** is copied next to it. Configs must be text.
- No personal data in configs (user names, paths, tokens).
- When importing someone else's theme, keep their license and credit them in `author`.
- Official library: open license, GitHub user, descriptions in both languages, a cover screenshot. Test apply and undo on a clean Windows 11 VM.

## Imported themes

*Import a theme* accepts a GitHub link (`https://github.com/user/repo` or a folder inside it). Only `theme.yaml` and the files it references are downloaded. The theme is marked **unverified** and the full plan is shown before anything is applied.
