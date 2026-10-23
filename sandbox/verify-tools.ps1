<#
  Runs INSIDE Windows Sandbox. For every tool in tools.json it:
    1. installs it with winget (the exact id from the manifest)
    2. checks the install paths declared in the manifest, and searches for the real ones
    3. starts it the way the manifest says, and records the real process names that appear
    4. records which config files appeared (to confirm config paths)
    5. runs the stop command and the widget quick actions from the manifest
    6. checks the PowerShell profile snippet (tools that use one)
    7. uninstalls it and checks the declared paths disappear
  Output: C:\WinRiceTest\report\report.json and report.txt  (send these back).

  Usage: verify-tools.ps1 [-Only glazewm,zebar]
#>
param([string[]]$Only)
$Root = "C:\WinRiceTest"
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"
$tools = Get-Content "$Root\tools.json" -Raw | ConvertFrom-Json
if ($Only) { $tools = $tools | Where-Object { $Only -contains $_.id } }
$out = Join-Path $Root "report"
New-Item -ItemType Directory -Force $out | Out-Null

function Expand([string]$s) { [Environment]::ExpandEnvironmentVariables($s) }
function Refresh-Path {
  $m = [Environment]::GetEnvironmentVariable("Path", "Machine"); $u = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = "$m;$u;$env:LOCALAPPDATA\Microsoft\WindowsApps;$env:LOCALAPPDATA\Microsoft\WinGet\Links"
}
function Find-Exe([string]$name) {
  $roots = @($env:ProgramFiles, ${env:ProgramFiles(x86)}, "$env:LOCALAPPDATA\Programs", "$env:LOCALAPPDATA\Microsoft\WinGet", "$env:LOCALAPPDATA\Microsoft\WindowsApps", $env:LOCALAPPDATA, $env:APPDATA) | Where-Object { $_ -and (Test-Path $_) }
  $found = foreach ($r in $roots) {
    Get-ChildItem -Path $r -Filter $name -Recurse -Depth 5 -File -ErrorAction SilentlyContinue | Select-Object -First 3 -ExpandProperty FullName
  }
  @($found | Select-Object -Unique)
}
function Proc-Names { @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object { $_.ProcessName + ".exe" } | Sort-Object -Unique) }
function Recent-Files([datetime]$since) {
  $noise = "\\Microsoft\\|\\Temp\\|\\WinRice|\\Packages\\|\\WinGet\\|CrashDumps|\\INetCache\\|\\WinRiceTest\\|\\Windows\\|\\Google\\|\\Edge\\"
  $roots = @($env:USERPROFILE, $env:APPDATA, $env:LOCALAPPDATA)
  $files = foreach ($r in $roots) {
    Get-ChildItem -Path $r -Recurse -Depth 3 -File -ErrorAction SilentlyContinue |
      Where-Object { $_.LastWriteTime -gt $since -and $_.FullName -notmatch $noise } | Select-Object -First 30 -ExpandProperty FullName
  }
  @($files | Select-Object -Unique | Select-Object -First 40)
}
function Run-Cmd([string]$program, [string[]]$arguments, [int]$timeoutSec = 20) {
  try {
    $p = Start-Process -FilePath $program -ArgumentList $arguments -PassThru -WindowStyle Hidden -RedirectStandardOutput "$out\_o.txt" -RedirectStandardError "$out\_e.txt"
    $null = $p.Handle   # keeps the handle so ExitCode is readable (Windows PowerShell 5.1 quirk)
    if (-not $p.WaitForExit($timeoutSec * 1000)) { $p.Kill(); return @{ exit = "timeout"; output = "" } }
    $p.WaitForExit()
    return @{ exit = $p.ExitCode; output = ("" + (Get-Content "$out\_o.txt" -Raw -ErrorAction SilentlyContinue) + (Get-Content "$out\_e.txt" -Raw -ErrorAction SilentlyContinue)).Trim() }
  } catch { return @{ exit = "error"; output = "$_" } }
}
function Split-Cmd([string]$cmd) { $parts = $cmd -split "\s+", 2; @{ program = $parts[0]; args = if ($parts.Count -gt 1) { @($parts[1] -split "\s+") } else { @() } } }

$report = @()
foreach ($t in $tools) {
  Write-Host "=== $($t.id) ($($t.install.id))" -ForegroundColor Cyan
  $r = [ordered]@{ id = $t.id; winget_id = $t.install.id }

  $t0 = Get-Date
  $inst = Run-Cmd "winget" @("install", "--id", $t.install.id, "-e", "--silent", "--accept-package-agreements", "--accept-source-agreements", "--source", "winget", "--disable-interactivity") 900
  $r.install_exit = $inst.exit
  Refresh-Path

  $r.declared_paths = @($t.detect.paths | ForEach-Object { [ordered]@{ path = $_; exists = [bool](Test-Path (Expand $_)) } })

  # real locations of the programs the manifest mentions
  $names = @()
  if ($t.detect.process) { $names += $t.detect.process }
  if ($t.activate.start) { $names += (Split-Cmd $t.activate.start).program }
  if ($t.deactivate.stop) { $names += (Split-Cmd $t.deactivate.stop).program }
  $r.found_exes = [ordered]@{}
  foreach ($n in ($names | Select-Object -Unique)) { $r.found_exes[$n] = @(Find-Exe $n) }
  $cmd = Get-Command ($t.detect.process) -ErrorAction SilentlyContinue
  $r.on_path = if ($cmd) { $cmd.Source } else { $null }

  # start it
  if ($t.activate.start) {
    $s = Split-Cmd $t.activate.start
    $exe = $null
    foreach ($d in $r.declared_paths) { if ($d.exists -and (Split-Path $d.path -Leaf) -ieq $s.program) { $exe = Expand $d.path } }
    if (-not $exe -and $r.found_exes[$s.program].Count -gt 0) { $exe = $r.found_exes[$s.program][0] }
    if (-not $exe) { $exe = $s.program }
    $r.start_used = "$exe $($s.args -join ' ')"
    $before = Proc-Names
    $since = Get-Date
    try { Start-Process -FilePath $exe -ArgumentList $s.args -WindowStyle Hidden } catch { $r.start_error = "$_" }
    Start-Sleep -Seconds 10
    $after = Proc-Names
    $r.new_processes = @($after | Where-Object { $before -notcontains $_ })
    $r.expected_process_running = [bool]($t.detect.process -and ($after -contains $t.detect.process))
    $r.new_config_files = Recent-Files $since

    # widget quick actions (while the tool runs)
    $r.quick_actions = @()
    foreach ($q in @($t.widget.quick_actions)) {
      if (-not $q) { continue }
      $qc = Split-Cmd $q.command
      $prog = if ($r.found_exes[$qc.program].Count -gt 0) { $r.found_exes[$qc.program][0] } else { $qc.program }
      $res = Run-Cmd $prog $qc.args
      $r.quick_actions += [ordered]@{ id = $q.id; command = $q.command; exit = $res.exit; output = $res.output }
    }

    # stop it the way the manifest says
    if ($t.deactivate.stop) {
      $sc = Split-Cmd $t.deactivate.stop
      $prog = if ($r.found_exes[$sc.program].Count -gt 0) { $r.found_exes[$sc.program][0] } else { $sc.program }
      $res = Run-Cmd $prog $sc.args
      Start-Sleep -Seconds 4
      $r.stop = [ordered]@{ command = $t.deactivate.stop; exit = $res.exit; output = $res.output; process_gone = -not ((Proc-Names) -contains $t.detect.process) }
    }
    # whatever is left: kill by process name, like WinRice does
    if ($t.detect.process) { Run-Cmd "taskkill" @("/IM", $t.detect.process, "/F") 10 | Out-Null }
    foreach ($p in $r.new_processes) { Run-Cmd "taskkill" @("/IM", $p, "/F") 10 | Out-Null }
  }

  # PowerShell profile snippet
  if ($t.activate.profile_snippet) {
    $sn = $t.activate.profile_snippet.content
    $res = Run-Cmd "powershell" @("-NoProfile", "-Command", "try { $sn; 'SNIPPET_OK' } catch { 'SNIPPET_ERROR: ' + `$_ }") 40
    $r.profile_snippet = [ordered]@{ content = $sn; exit = $res.exit; output = $res.output }
    $r.profile_dirs = @($t.activate.profile_snippet.files | ForEach-Object { [ordered]@{ file = $_; parent_exists = [bool](Test-Path (Split-Path (Expand $_))) } })
  }

  # configs declared by the manifest
  if ($t.config.paths) { $r.config_paths = @($t.config.paths | ForEach-Object { [ordered]@{ path = $_; exists = [bool](Test-Path (Expand $_)) } }) }

  # uninstall
  $un = Run-Cmd "winget" @("uninstall", "--id", $t.install.id, "-e", "--silent", "--source", "winget", "--disable-interactivity") 600
  $r.uninstall_exit = $un.exit
  $r.paths_after_uninstall = @($t.detect.paths | ForEach-Object { [ordered]@{ path = $_; still_exists = [bool](Test-Path (Expand $_)) } })

  $report += [pscustomobject]$r
  # save after every tool so a crash does not lose what was done
  $report | ConvertTo-Json -Depth 8 | Set-Content "$out\report.json" -Encoding UTF8
}

# human-readable summary
$txt = foreach ($r in $report) {
  $okPaths = @($r.declared_paths | Where-Object { $_.exists }).Count
  "[{0}] install={1}  declared paths found={2}/{3}  process running={4}  uninstall={5}" -f $r.id, $r.install_exit, $okPaths, @($r.declared_paths).Count, $r.expected_process_running, $r.uninstall_exit
  foreach ($k in $r.found_exes.Keys) { "    real location of {0}: {1}" -f $k, (($r.found_exes[$k]) -join " | ") }
  if ($r.new_processes) { "    processes started: " + ($r.new_processes -join ", ") }
  if ($r.new_config_files) { "    new files after start: " + ($r.new_config_files -join " | ") }
  foreach ($q in @($r.quick_actions)) { "    quick action {0}: exit={1} {2}" -f $q.id, $q.exit, $q.output }
  if ($r.stop) { "    stop: exit={0} process_gone={1} {2}" -f $r.stop.exit, $r.stop.process_gone, $r.stop.output }
  if ($r.profile_snippet) { "    profile snippet: " + $r.profile_snippet.output }
  ""
}
$txt | Set-Content "$out\report.txt" -Encoding UTF8
Write-Host ""
Write-Host "Done. Report: $out\report.txt and report.json" -ForegroundColor Green
