# Runs INSIDE Windows Sandbox: sets up winget, then runs `winrice.exe --selftest` (the real engine) and leaves the report.
$Root = "C:\WinRiceTest"
New-Item -ItemType Directory -Force "$Root\selftest" | Out-Null
"started $(Get-Date -Format s)" | Set-Content "$Root\selftest\RUNNING.txt"
& "$Root\bootstrap.ps1"
$env:Path += ";$env:LOCALAPPDATA\Microsoft\WindowsApps"

# The winget package source is not always reachable right after the first start (error 0x8A15003B):
# reset it and wait until a real lookup works, then run the engine.
$ready = $false
for ($i = 1; $i -le 8 -and -not $ready; $i++) {
  "winget source attempt $i" | Add-Content "$Root\selftest\stage.txt"
  winget source reset --force 2>&1 | Out-Null
  winget source update 2>&1 | Out-Null
  $out = winget show --id Flow-Launcher.Flow-Launcher -e --source winget --accept-source-agreements 2>&1 | Out-String
  if ($LASTEXITCODE -eq 0 -and $out -match "Flow") { $ready = $true } else { Start-Sleep -Seconds 15 }
}
"winget ready: $ready" | Add-Content "$Root\selftest\stage.txt"

$p = Start-Process "$Root\app\winrice.exe" -ArgumentList @("--selftest", "$Root\selftest\report.json") -PassThru -Wait -WindowStyle Hidden
"exit $($p.ExitCode)" | Set-Content "$Root\selftest\exit.txt"
Remove-Item "$Root\selftest\RUNNING.txt" -ErrorAction SilentlyContinue
"finished $(Get-Date -Format s)" | Set-Content "$Root\selftest\DONE.txt"
Write-Host "SELFTEST DONE. You can close this window." -ForegroundColor Green
