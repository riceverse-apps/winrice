# Runs INSIDE Windows Sandbox when the kit was prepared with -Auto: does everything unattended
# (set up winget, verify every tool, collect the results) and leaves DONE.txt in the shared folder.
$Root = "C:\WinRiceTest"
$ErrorActionPreference = "Continue"
New-Item -ItemType Directory -Force "$Root\results" | Out-Null
"started $(Get-Date -Format s)" | Set-Content "$Root\results\RUNNING.txt"

& "$Root\bootstrap.ps1"
& "$Root\verify-tools.ps1"
& "$Root\collect.ps1"

Remove-Item "$Root\results\RUNNING.txt" -ErrorAction SilentlyContinue
"finished $(Get-Date -Format s)" | Set-Content "$Root\results\DONE.txt"
Write-Host ""
Write-Host "ALL DONE. The results are in the shared folder (sandbox\stage\results). You can close this window." -ForegroundColor Green
