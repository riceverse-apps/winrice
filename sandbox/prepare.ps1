<#
  Prepares the Windows Sandbox test kit on THIS computer (it changes nothing outside the repo):
    - copies the built WinRice app and the test scripts into sandbox\stage
    - exports the tool manifests to JSON
    - writes sandbox\winrice.wsb with the right folder path
  Then double-click sandbox\winrice.wsb (or pass -Launch).

  Usage:  powershell -ExecutionPolicy Bypass -File sandbox\prepare.ps1 [-Exe <path to winrice.exe>] [-Launch]
#>
param(
  [string]$Exe,
  [switch]$Launch
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$stage = Join-Path $PSScriptRoot "stage"

if (-not $Exe) {
  foreach ($c in @("app\src-tauri\target\release\winrice.exe", "app\src-tauri\target\debug\winrice.exe")) {
    $p = Join-Path $repo $c
    if (Test-Path $p) { $Exe = $p; break }
  }
}
if (-not $Exe -or -not (Test-Path $Exe)) { throw "winrice.exe not found. Build it first: npm --prefix app run tauri -- build --no-bundle" }

if (-not (Get-Command WindowsSandbox.exe -ErrorAction SilentlyContinue)) {
  Write-Warning "Windows Sandbox is not enabled on this PC. In an ADMIN PowerShell run:  Enable-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM -All   and restart."
}

New-Item -ItemType Directory -Force $stage | Out-Null
New-Item -ItemType Directory -Force (Join-Path $stage "app") | Out-Null
Copy-Item $Exe (Join-Path $stage "app\winrice.exe") -Force
foreach ($f in "bootstrap.ps1", "verify-tools.ps1", "collect.ps1") { Copy-Item (Join-Path $PSScriptRoot $f) $stage -Force }

# the in-sandbox shortcuts (.cmd avoids execution-policy prompts)
Set-Content (Join-Path $stage "1-VERIFY-TOOLS.cmd") "@echo off`r`npowershell -NoProfile -ExecutionPolicy Bypass -File C:\WinRiceTest\verify-tools.ps1`r`npause" -Encoding ASCII
Set-Content (Join-Path $stage "2-RUN-WINRICE.cmd") "@echo off`r`nstart `"`" C:\WinRiceTest\app\winrice.exe" -Encoding ASCII
Set-Content (Join-Path $stage "3-COLLECT-RESULTS.cmd") "@echo off`r`npowershell -NoProfile -ExecutionPolicy Bypass -File C:\WinRiceTest\collect.ps1`r`npause" -Encoding ASCII

Push-Location $repo
try { node sandbox\export-tools.mjs (Join-Path $stage "tools.json") } finally { Pop-Location }

$wsb = @"
<Configuration>
  <VGpu>Enable</VGpu>
  <Networking>Enable</Networking>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$stage</HostFolder>
      <SandboxFolder>C:\WinRiceTest</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>powershell -NoProfile -ExecutionPolicy Bypass -File C:\WinRiceTest\bootstrap.ps1</Command>
  </LogonCommand>
</Configuration>
"@
$wsbPath = Join-Path $PSScriptRoot "winrice.wsb"
Set-Content $wsbPath $wsb -Encoding UTF8
Write-Host "Ready. Kit in: $stage"
Write-Host "Open: $wsbPath"
if ($Launch) { Start-Process $wsbPath }
