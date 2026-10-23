# Translating

English is the base language and Brazilian Portuguese (`pt-BR`) is complete. More languages are welcome.

## What to translate

1. **Interface:** `locales/<code>.json` (copy `locales/en.json`). `npm run validate` fails if a language lacks keys that English has, or has extra keys.
2. **Tools:** every `i18n` field in `catalog/tools/*/manifest.yaml` (`summary`, `changes_on_screen`, `shortcuts`, `risks`, `how_to_undo`, `settings[].label`, `widget.quick_actions[].label`) and `docs/<code>.md`.
3. **Themes:** `description`.
4. **Glossary:** `catalog/glossary/glossary.yaml`.

When a translation is missing the app falls back to English.

## Adding a language to the app

1. Add `locales/<code>.json`.
2. Import it and add it to `messages` in `app/src/i18n/index.ts`, and add `{ code, label }` to `LOCALES` there.
3. Make sure `npm run validate` and `npm --prefix app run build` pass.

The manifest schema currently requires `en` and `pt-BR` in every text field, and extra languages are allowed next to them.

## Style

Write for beginners: short sentences, no unexplained jargon. The first time a technical word appears, add it to the glossary instead of explaining it inline. Keep product names (GlazeWM, Windhawk) untranslated.
