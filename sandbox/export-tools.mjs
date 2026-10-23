// Dumps every tool manifest as JSON so the PowerShell verification script can read it
// (Windows PowerShell 5.1 has no YAML parser).   node sandbox/export-tools.mjs <output.json>
import { readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import YAML from "yaml";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dir = join(root, "catalog", "tools");
const tools = readdirSync(dir)
  .filter((id) => statSync(join(dir, id)).isDirectory())
  .map((id) => YAML.parse(readFileSync(join(dir, id, "manifest.yaml"), "utf8")));
writeFileSync(process.argv[2] ?? "tools.json", JSON.stringify(tools, null, 2));
console.log(`exported ${tools.length} tools`);
