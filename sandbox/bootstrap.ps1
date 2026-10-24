# Runs INSIDE Windows Sandbox at start-up. Makes sure winget and the Visual C++ runtime exist.
$Root = "C:\WinRiceTest"
$ErrorActionPreference = "Continue"
Start-Transcript -Path "$Root\bootstrap.log" -Force | Out-Null

function Has-Winget { [bool](Get-Command winget -ErrorAction SilentlyContinue) }

if (-not (Has-Winget)) {
  Write-Host "winget not found: installing App Installer from Microsoft (aka.ms)..."
  $tmp = Join-Path $env:TEMP "wg"
  New-Item -ItemType Directory -Force $tmp | Out-Null
  $ProgressPreference = "SilentlyContinue"
  try {
    # The winget release ships the exact framework packages it needs (VCLibs, UI.Xaml, Windows App
    # Runtime...); a clean Sandbox has none of them, and the newest winget will not install without.
    $base = "https://github.com/microsoft/winget-cli/releases/latest/download"
    Invoke-WebRequest "$base/DesktopAppInstaller_Dependencies.zip" -OutFile "$tmp\deps.zip"
    Invoke-WebRequest "$base/Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle" -OutFile "$tmp\winget.msixbundle"
    Expand-Archive "$tmp\deps.zip" -DestinationPath "$tmp\deps" -Force
    # install every x64 dependency; "already installed / newer version present" is fine
    Get-ChildItem "$tmp\deps" -Recurse -Filter *.appx | Where-Object { $_.FullName -match '\\x64\\' } | ForEach-Object {
      try { Add-AppxPackage -Path $_.FullName -ErrorAction Stop; Write-Host "  dependency installed: $($_.Name)" }
      catch { Write-Host "  dependency skipped: $($_.Name) ($($_.Exception.Message.Split([char]10)[0]))" }
    }
    Add-AppxPackage -Path "$tmp\winget.msixbundle" -ErrorAction Stop
  } catch { Write-Warning "Could not install winget automatically: $_" }
  $env:Path += ";$env:LOCALAPPDATA\Microsoft\WindowsApps"
}

if (Has-Winget) {
  winget source update --source winget 2>&1 | Out-Null
  # WinRice is built with the MSVC toolchain; the Sandbox image may not have the runtime.
  if (-not (Test-Path "$env:windir\System32\vcruntime140.dll")) {
    winget install --id Microsoft.VCRedist.2015+.x64 -e --silent --accept-package-agreements --accept-source-agreements --source winget
  }
} else {
  Write-Warning "winget is NOT available: tools cannot be installed in this session."
}

# Desktop shortcuts to the three steps
$desk = [Environment]::GetFolderPath("Desktop")
foreach ($n in "1-VERIFY-TOOLS.cmd", "2-RUN-WINRICE.cmd", "3-COLLECT-RESULTS.cmd") { Copy-Item "$Root\$n" $desk -Force }

Write-Host ""
Write-Host "Ready. On the desktop:"
Write-Host "  1-VERIFY-TOOLS   installs each tool and records where things really are (takes a while)"
Write-Host "  2-RUN-WINRICE    opens WinRice for the manual checklist"
Write-Host "  3-COLLECT-RESULTS copies everything to the shared folder before you close the Sandbox"
Stop-Transcript | Out-Null
