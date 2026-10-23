# Runs INSIDE Windows Sandbox before you close it: copies everything worth keeping into the shared folder.
$Root = "C:\WinRiceTest"
$dest = Join-Path $Root "results"
New-Item -ItemType Directory -Force $dest | Out-Null

# WinRice's own data: database (history with errors), backups, imported/saved themes
$data = Join-Path $env:APPDATA "WinRice"
if (Test-Path $data) { Copy-Item $data (Join-Path $dest "WinRice-data") -Recurse -Force }

# files WinRice may have edited
foreach ($f in @("$env:USERPROFILE\Documents\PowerShell\Microsoft.PowerShell_profile.ps1",
                 "$env:USERPROFILE\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1")) {
  if (Test-Path $f) { Copy-Item $f (Join-Path $dest ("profile-" + ($f -replace '[\\:]', '_'))) -Force }
}

# autostart entries
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" 2>&1 | Set-Content (Join-Path $dest "run-key.txt")
# running processes and wallpaper
Get-Process | Sort-Object ProcessName | Select-Object ProcessName, Id | Out-String | Set-Content (Join-Path $dest "processes.txt")
reg query "HKCU\Control Panel\Desktop" /v WallPaper 2>&1 | Set-Content (Join-Path $dest "wallpaper.txt")
Get-ComputerInfo -Property WindowsProductName, OsBuildNumber, OsVersion 2>$null | Out-String | Set-Content (Join-Path $dest "windows.txt")

# verification report, if it was run
if (Test-Path "$Root\report") { Copy-Item "$Root\report" (Join-Path $dest "report") -Recurse -Force }
Write-Host "Copied to $dest  (on your PC: sandbox\stage\results)" -ForegroundColor Green
