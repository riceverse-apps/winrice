// Validates locales parity, tool manifests and themes; writes catalog/index.json.
import { readFileSync, readdirSync, existsSync, writeFileSync, statSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv";
import addFormats from "ajv-formats";
import YAML from "yaml";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const errors = [];
const fail = (m) => errors.push(m);
const json = (p) => JSON.parse(readFileSync(join(root, p), "utf8"));

const ajv = new Ajv({ allErrors: true });
addFormats(ajv);
const toolSchema = ajv.compile(json("schemas/tool.schema.json"));
const themeSchema = ajv.compile(json("schemas/theme.schema.json"));

// 1. Locale key parity (pt-BR must cover 100% of en).
const flat = (o, p = "") =>
  Object.entries(o).flatMap(([k, v]) => (typeof v === "object" ? flat(v, p + k + ".") : [p + k]));
const en = new Set(flat(json("locales/en.json")));
const pt = new Set(flat(json("locales/pt-BR.json")));
for (const k of en) if (!pt.has(k)) fail(`locales: pt-BR is missing "${k}"`);
for (const k of pt) if (!en.has(k)) fail(`locales: en is missing "${k}"`);

// 2. Entries.
function entries(dir, file) {
  const base = join(root, dir);
  if (!existsSync(base)) return [];
  return readdirSync(base)
    .filter((d) => statSync(join(base, d)).isDirectory())
    .map((d) => ({ dir: d, path: join(base, d, file), data: existsSync(join(base, d, file)) ? YAML.parse(readFileSync(join(base, d, file), "utf8")) : null }));
}

const tools = entries("catalog/tools", "manifest.yaml");
const themes = entries("catalog/themes", "theme.yaml");
const toolIds = new Set();

for (const t of tools) {
  if (!t.data) { fail(`tools/${t.dir}: manifest.yaml missing`); continue; }
  if (!toolSchema(t.data)) fail(`tools/${t.dir}: ${ajv.errorsText(toolSchema.errors)}`);
  if (t.data.id !== t.dir) fail(`tools/${t.dir}: id "${t.data.id}" must match the folder name`);
  if (toolIds.has(t.data.id)) fail(`tools/${t.dir}: duplicate id`);
  toolIds.add(t.data.id);
  for (const lang of ["en", "pt-BR"])
    if (!existsSync(join(root, "catalog/tools", t.dir, "docs", lang + ".md"))) fail(`tools/${t.dir}: docs/${lang}.md missing`);
}
for (const t of tools)
  for (const c of t.data?.conflicts ?? [])
    if (!toolIds.has(c)) fail(`tools/${t.dir}: conflicts references unknown tool "${c}"`);

// Glossary: every term needs both languages.
const glossary = YAML.parse(readFileSync(join(root, "catalog/glossary/glossary.yaml"), "utf8")) ?? [];
const termIds = new Set();
for (const g of glossary) {
  if (!g.id || termIds.has(g.id)) fail(`glossary: missing or duplicate id "${g.id}"`);
  termIds.add(g.id);
  for (const f of ["term", "definition"])
    for (const l of ["en", "pt-BR"]) if (!g[f]?.[l]) fail(`glossary/${g.id}: ${f}.${l} missing`);
}

// Themes are data only: no executables or scripts, no oversized bundles.
const ALLOWED = new Set(["yaml", "yml", "json", "jsonc", "toml", "css", "ini", "md", "txt", "png", "jpg", "jpeg", "webp", "gif", "bmp"]);
function walk(dir, base = dir) {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p, base) : [p.slice(base.length + 1).replaceAll("\\", "/")];
  });
}
for (const t of themes) {
  const base = join(root, "catalog/themes", t.dir);
  let total = 0;
  for (const f of walk(base)) {
    const ext = f.split(".").pop().toLowerCase();
    if (!ALLOWED.has(ext)) fail(`themes/${t.dir}: forbidden file type ${f}`);
    total += statSync(join(base, f)).size;
  }
  if (total > 8 * 1024 * 1024) fail(`themes/${t.dir}: larger than 8 MB`);
}

for (const t of themes) {
  if (!t.data) { fail(`themes/${t.dir}: theme.yaml missing`); continue; }
  if (t.data.id !== t.dir) fail(`themes/${t.dir}: id "${t.data.id}" must match the folder name`);
  if (!themeSchema(t.data)) fail(`themes/${t.dir}: ${ajv.errorsText(themeSchema.errors)}`);
  for (const c of t.data.components ?? [])
    if (!toolIds.has(c.tool)) fail(`themes/${t.dir}: unknown tool "${c.tool}"`);
}

if (errors.length) {
  console.error(errors.map((e) => "✗ " + e).join("\n"));
  process.exit(1);
}

// The app downloads this index to get new tools/themes without an update; every file
// is listed with its SHA-256 so the app can verify what it downloads.
// The repository stores text with LF (see .gitattributes) and the app downloads those bytes,
// so hash text files with LF even when this runs on a Windows checkout that has CRLF.
const BINARY = /\.(png|jpe?g|webp|gif|bmp|ico)$/i;
const sha = (path) => {
  let bytes = readFileSync(path);
  if (!BINARY.test(path)) bytes = Buffer.from(bytes.toString("utf8").replaceAll("\r\n", "\n"), "utf8");
  return createHash("sha256").update(bytes).digest("hex");
};
const entry = (kind, dir) => {
  const base = join(root, "catalog", kind, dir);
  return {
    id: dir,
    files: walk(base).sort().map((f) => ({ path: f, sha256: sha(join(base, f)) })),
  };
};
writeFileSync(
  join(root, "catalog/index.json"),
  JSON.stringify({ version: 1, tools: tools.map((t) => entry("tools", t.dir)), themes: themes.map((t) => entry("themes", t.dir)) }, null, 2) + "\n",
);
console.log(`OK: ${tools.length} tools, ${themes.length} themes, ${en.size} locale keys`);
