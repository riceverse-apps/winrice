<#
  Updates WinRice from GitHub, rebuilds it only when something changed, and opens it.

    powershell -ExecutionPolicy Bypass -File scripts\update-and-run.ps1 [-NoPull] [-Force] [-NoLaunch] [-Shortcut]

    -NoPull    do not fetch from GitHub (use the code as it is on disk)
    -Force     rebuild even if nothing changed
    -NoLaunch  update and build, but do not open the app
    -Shortcut  also create a Start Menu entry "WinRice (update and open)"

  Any running WinRice is closed first (a new build cannot replace a running .exe).
#>
param(
  [switch]$NoPull,
  [switch]$Force,
  [switch]$NoLaunch,
  [switch]$Shortcut
)
# Windows PowerShell 5.1 turns a program's stderr text (tauri prints progress there) into errors under
# "Stop", so failures are detected through exit codes instead.
$ErrorActionPreference = "Continue"
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
$exe = Join-Path $repo "app\src-tauri\target\release\winrice.exe"
$stampFile = Join-Path $repo "app\src-tauri\target\.winrice-build-stamp"

function Step($text) { Write-Host "==> $text" -ForegroundColor Cyan }
function Fail($text) { Write-Host "ERROR: $text" -ForegroundColor Red; exit 1 }

# Rust installed after this terminal was opened is not on its PATH yet.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) { $env:Path += ";$cargoBin" }
foreach ($tool in "git", "node", "npm", "cargo") {
  if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { Fail "'$tool' was not found. Install it (see README) and open a new terminal." }
}

# ---- 1. update from GitHub
if (-not $NoPull) {
  Step "Checking GitHub for updates"
  $env:GIT_TERMINAL_PROMPT = "0"
  # Only changes to tracked files block a pull (untracked files never do).
  $dirty = [bool](git status --porcelain --untracked-files=no)
  git fetch origin 2>&1 | Out-Null
  if ($LASTEXITCODE -ne 0) {
    Write-Warning "Could not reach GitHub (offline or not signed in). Using the code that is already here."
  } else {
    $behind = [int](git rev-list --count "HEAD..origin/main" 2>$null)
    if ($behind -gt 0) {
      if ($dirty) {
        Write-Warning "$behind new commit(s) on GitHub, but you have local changes, so I did not pull. Commit or stash them, or use -NoPull."
      } else {
        Step "Pulling $behind new commit(s)"
        git pull --ff-only origin main 2>&1 | ForEach-Object { Write-Host "    $_" }
        if ($LASTEXITCODE -ne 0) { Fail "git pull failed (the local history differs from GitHub). Resolve it with git, then run this again." }
      }
    } else {
      Write-Host "    already up to date"
    }
  }
}

# ---- 2. does it need a rebuild?
# The fingerprint is the commit plus the content of any uncommitted change.
$head = (git rev-parse HEAD).Trim()
$diff = (git diff HEAD | Out-String)
$sha = [System.Security.Cryptography.SHA256]::Create()
$fingerprint = $head + ":" + ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($diff))) -replace "-", "")
$last = if (Test-Path $stampFile) { (Get-Content $stampFile -Raw).Trim() } else { "" }
$needBuild = $Force -or -not (Test-Path $exe) -or ($fingerprint -ne $last)

# ---- 3. close a running WinRice
$running = Get-Process winrice -ErrorAction SilentlyContinue
if ($running -and ($needBuild -or -not $NoLaunch)) {
  Step "Closing the running WinRice"
  $running | Stop-Process -Force
  Start-Sleep -Milliseconds 800
}

# ---- 4. build
if ($needBuild) {
  Step "Installing dependencies (only what is missing)"
  if (-not (Test-Path "node_modules")) { npm install --silent 2>&1 | Out-Null }
  if (-not (Test-Path "app\node_modules")) { npm install --prefix app --silent 2>&1 | Out-Null }

  Step "Validating the catalog"
  node scripts\validate-catalog.mjs | Out-Host
  if ($LASTEXITCODE -ne 0) { Fail "The catalog is not valid (see above)." }

  Step "Building WinRice (first time takes a few minutes, later ones are quick)"
  npm --prefix app run tauri -- build --no-bundle 2>&1 | ForEach-Object { if ($_ -match "error|warning: unused|Finished|Built") { Write-Host "    $_" } }
  if ($LASTEXITCODE -ne 0 -or -not (Test-Path $exe)) { Fail "The build failed. Run: npm --prefix app run tauri -- build --no-bundle   to see the full output." }
  Set-Content $stampFile $fingerprint
} else {
  Step "Nothing changed since the last build"
}

# ---- 5. optional Start Menu entry
if ($Shortcut) {
  $lnk = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\WinRice (update and open).lnk"
  $s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
  $s.TargetPath = "powershell.exe"
  $s.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
  $s.WorkingDirectory = $repo
  $s.IconLocation = "$exe,0"
  $s.Save()
  Step "Start Menu entry created: search for 'WinRice (update and open)'"
}

# ---- 6. open it
if (-not $NoLaunch) {
  Step "Opening WinRice"
  Start-Process $exe
}
