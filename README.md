# WinRice

Customize Windows 11 with one click: a catalog of community "rice" tools (tiling window managers, status bars, shell mods, terminal prompts, launchers), a theme library and a quick-access widget. Every change shows its plan first, is backed up, and can be undone.

[Português (Brasil)](README.pt-BR.md)

> **Status: in development, no public release yet.** The code builds and the automated tests pass. The installs, paths, start and stop commands of 19 of the 20 tools were verified in a clean Windows 11 Sandbox ([what was verified](docs/verified-tools.md)), but WinRice's own flows (toggles, themes, undo, widget) have **not** been exercised there yet. Do not run it on a machine you cannot afford to experiment on.

## What it does

- **Tool catalog** (20 tools): install, turn on/off and uninstall with a toggle. Each tool has plain-language docs in English and Portuguese.
- **Safe by design**: every change goes through one Transaction Engine (plan → backup → apply → verify → commit or roll back). If a step fails, everything already done is undone.
- **Try mode**: apply for 30 seconds; if you do not confirm, it reverts by itself. Emergency shortcut: `Ctrl + Alt + Shift + R`.
- **Conflicts and compatibility**: tools that clash are blocked with an explanation (or switched off for you); tools that do not support your Windows build are disabled. An Explorer watchdog reverts a crash loop.
- **Theme library**: 5 official themes, partial or full apply, undo, import from a GitHub link, save your setup as a theme, export `.zip`, palette sync, and a guided flow to send a theme to the official library.
- **Profiles and "Pause customizations"**: switch the set of active tools in one click.
- **Widget**: tray flyout and floating widget with quick toggles, profiles and live hardware info (CPU, RAM, disks, network, battery). Temperatures, fans and GPU load need a privileged helper that is **not** built yet.
- **History with undo** (newest change first), favorites, light/dark/system mode and accent color.
- **No telemetry.** Nothing leaves your computer except the downloads you ask for (GitHub, winget).

## Development

Requirements: Node 20+, Rust (stable, MSVC) and the Visual Studio C++ Build Tools, WebView2 (included in Windows 11).

```bash
npm install                          # catalog tooling
npm run validate                     # locales, manifests, themes, glossary; writes catalog/index.json
npm install --prefix app
npm --prefix app run dev             # web UI only, http://localhost:1420
npm --prefix app run tauri -- dev    # desktop app
cargo test --manifest-path app/src-tauri/Cargo.toml
```

The tests never touch your machine: the Transaction Engine runs against an in-memory fake of the operating system. Testing the real thing is done in a VM or Windows Sandbox (see [CONTRIBUTING](CONTRIBUTING.md)).

## Repository layout

| Folder | Content |
|---|---|
| `app/` | Tauri 2 (Rust) + Vue 3 app |
| `catalog/tools/` | one folder per tool: `manifest.yaml`, `docs/en.md`, `docs/pt-BR.md` |
| `catalog/themes/` | official themes |
| `catalog/glossary/` | jargon explained in plain language |
| `schemas/` | JSON Schemas for manifests and themes |
| `locales/` | `en.json`, `pt-BR.json` |
| `branding/` | logos and icons |
| `docs/` | project documentation |
| `packaging/`, `site/` | winget manifests, landing page |

## Documentation

[Architecture](docs/architecture.md) · [Adding a tool](docs/adding-a-tool.md) · [Creating a theme](docs/creating-a-theme.md) · [Manifest reference](docs/manifest-reference.md) · [Theme reference](docs/theme-reference.md) · [Translating](docs/translating.md) · [Publishing](docs/publishing.md) · [VM testing](docs/vm-testing.md) · [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md)

## License

MIT, see [LICENSE](LICENSE). Third-party tools keep their own licenses; WinRice never bundles their binaries, it installs them from their official sources.
