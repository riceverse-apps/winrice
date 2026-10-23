# Adding a tool

A tool is a folder in `catalog/tools/<tool-id>/`. No application code changes.

```
catalog/tools/<tool-id>/
  manifest.yaml
  docs/en.md
  docs/pt-BR.md
```

## Steps

1. **Research the official source.** Confirm the winget id (`winget show <id>`), the license, the Windows builds supported, where the config lives, how to start and stop it. Write down anything you could not confirm.
2. **Write `manifest.yaml`** (see the [manifest reference](manifest-reference.md)). Copy a similar tool as a starting point: `glazewm` (process + autostart), `oh-my-posh` (PowerShell profile block), `windows-terminal` (install only).
3. **Write the docs** in English and Portuguese, using the nine sections of the existing docs (what it is, who it is for, what changes on screen, first steps, shortcuts, customizations, common problems, how to undo, links).
4. **Declare conflicts on both sides.** If A conflicts with B, edit B's manifest too. CI checks this.
5. **Mark what you could not verify** with `# UNVERIFIED:` comments (install paths are the usual case).
6. `npm run validate` and `cargo test --manifest-path app/src-tauri/Cargo.toml`.
7. **Test in a VM / Windows Sandbox**: install, turn on, turn off, uninstall, and a forced failure in the middle (rollback). Note the result in the PR.

## Choosing how the tool is switched on

| Tool kind | Use |
|---|---|
| Runs as a background program | `detect.process`, `activate.start`, `activate.autostart`, optionally `deactivate.stop` |
| Changes your PowerShell prompt | `activate.profile_snippet` (a marked block is added to / removed from the profile) |
| Installs and does nothing until configured, or is too risky to toggle | omit `activate` / `deactivate`: the tool is install/uninstall only |

If the manifest cannot express something the tool needs, **extend the schema** (`schemas/tool.schema.json` and `app/src-tauri/src/manifest.rs`) and add a test. Do not add a special case for one tool.

## Optional extras

- `settings:` simple options shown in the Settings tab (numbers, switches, text, colors). Add `palette: accent` to a color option so *Sync colors* can recolor it.
- `widget:` `show_by_default` and `quick_actions` (extra buttons in the widget).
- `paid: true` for paid software, `restricted_license: true` when the license limits use (for example commercial use).
