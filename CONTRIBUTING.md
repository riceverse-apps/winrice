# Contributing to WinRice

Thank you for helping. Most contributions are **data**: a new tool, a new theme or a translation. You do not need to write Rust for those.

## Ground rules

- Code, commits and repository docs are in **English**. Everything the user sees goes through i18n, with **complete pt-BR**.
- **Never commit third-party binaries.** Tools are installed from their official source (winget, GitHub Releases, vendor site).
- A theme is **data only**: YAML/JSON/TOML/CSS/INI/text, images. Never programs or scripts. CI rejects anything else.
- Verify facts at the official source before writing a manifest: winget id, license, supported Windows builds, config paths. If you cannot verify something, mark it with a `# UNVERIFIED:` comment instead of guessing.
- Everything that changes the system goes through the Transaction Engine. No exceptions.
- **Test system changes in a VM or Windows Sandbox**, never on your main machine.

## Setup

```bash
npm install && npm install --prefix app
npm run validate                                   # must pass before every PR
cargo test --manifest-path app/src-tauri/Cargo.toml
npm --prefix app run build                         # type-check + build the UI
```

## Adding a tool

See [docs/adding-a-tool.md](docs/adding-a-tool.md). In short: `catalog/tools/<id>/manifest.yaml` + `docs/en.md` + `docs/pt-BR.md`, declare `conflicts` on **both** sides, run `npm run validate`, and test install / turn on / turn off / uninstall in a VM.

## Creating or sending a theme

See [docs/creating-a-theme.md](docs/creating-a-theme.md). The app has a guided flow: open one of **My themes** → *Send to the official library*. It checks the rules, prepares the folder `catalog/themes/<id>/` and a ready pull request text.

Requirements for the official library: open license, author GitHub user, description in English and pt-BR, a cover screenshot, only known tools, no personal data in configs, tested apply and undo on a clean Windows 11 VM.

## Translating

See [docs/translating.md](docs/translating.md).

## Pull requests

1. Fork, branch from `main`.
2. Keep one subject per PR (one tool, one theme, one fix).
3. Fill in the pull request template. Say what you tested and where (VM / real machine).
4. CI runs `npm run validate`, builds the app and runs the tests.

## Code style

- Rust: keep functions small, return errors as stable codes (`"conflict:a,b"`), never hard-code per-tool logic: extend the manifest schema instead.
- Vue/TS: strict TypeScript, colors only from the CSS tokens in `app/src/styles/tokens.css`, all text through `t()`.
- Add a test with every behavior change in the engine. Rollback must be tested by forcing a failure at **every** step (see `app/src-tauri/src/tests.rs`).

By contributing you agree that your contribution is licensed under the MIT license of this project.
