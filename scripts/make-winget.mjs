// Generates the winget manifests (schema 1.6.0) for a WinRice release.
//   node scripts/make-winget.mjs --version 0.1.0 --url <installer url> --sha256 <hash>
// Output: packaging/winget/out/manifests/r/RiceverseApps/WinRice/<version>/
import { mkdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const args = Object.fromEntries(
  process.argv.slice(2).reduce((acc, a, i, all) => (a.startsWith("--") ? [...acc, [a.slice(2), all[i + 1]]] : acc), []),
);
const { version, url, sha256 } = args;

const fail = (m) => {
  console.error("make-winget: " + m);
  process.exit(1);
};
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) fail("--version must look like 1.2.3");
if (!/^https:\/\/github\.com\/riceverse-apps\/winrice\/releases\/download\/.+\.exe$/.test(url ?? "")) fail("--url must be a GitHub release .exe of riceverse-apps/winrice");
if (!/^[0-9a-fA-F]{64}$/.test(sha256 ?? "")) fail("--sha256 must be 64 hex characters");

const id = "RiceverseApps.WinRice";
const header = (type) => `# yaml-language-server: $schema=https://aka.ms/winget-manifest.${type}.1.6.0.schema.json\n`;

const files = {
  [`${id}.yaml`]: header("version") + `PackageIdentifier: ${id}
PackageVersion: ${version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: 1.6.0
`,
  [`${id}.installer.yaml`]: header("installer") + `PackageIdentifier: ${id}
PackageVersion: ${version}
InstallerType: nullsoft
Scope: user
Installers:
  - Architecture: x64
    InstallerUrl: ${url}
    InstallerSha256: ${sha256.toUpperCase()}
ManifestType: installer
ManifestVersion: 1.6.0
`,
  [`${id}.locale.en-US.yaml`]: header("defaultLocale") + `PackageIdentifier: ${id}
PackageVersion: ${version}
PackageLocale: en-US
Publisher: Riceverse
PublisherUrl: https://github.com/riceverse-apps
PackageName: WinRice
PackageUrl: https://github.com/riceverse-apps/winrice
License: MIT
LicenseUrl: https://github.com/riceverse-apps/winrice/blob/main/LICENSE
ShortDescription: Customize Windows 11 with one click, with backup and undo for every change.
Description: A catalog of community customization tools (tiling window managers, status bars, shell mods, terminal prompts, launchers), a theme library and a quick-access widget. Every change is planned, backed up and reversible.
Moniker: winrice
Tags:
  - customization
  - rice
  - theme
  - windows-11
  - tiling
ManifestType: defaultLocale
ManifestVersion: 1.6.0
`,
};

const out = join(dirname(fileURLToPath(import.meta.url)), "..", "packaging", "winget", "out", "manifests", "r", "RiceverseApps", "WinRice", version);
mkdirSync(out, { recursive: true });
for (const [name, text] of Object.entries(files)) writeFileSync(join(out, name), text);
console.log(`winget manifests written to ${out}`);
