#requires -Version 7
# Robust resumable download: the CN mirror connection drops every few MB
# ("schannel: server closed abruptly"). Each attempt resumes from the last
# byte, so we loop until the local file matches the remote Content-Length.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$installRoot = 'C:\DevTools'
$flutterDir  = Join-Path $installRoot 'flutter'
$zipPath     = Join-Path $installRoot 'flutter_sdk.zip'
$curl        = "$env:SystemRoot\System32\curl.exe"

New-Item -ItemType Directory -Path $installRoot -Force | Out-Null

$manifest = Invoke-RestMethod -Uri 'https://storage.googleapis.com/flutter_infra_release/releases/releases_windows.json' -TimeoutSec 60
$stableHash = $manifest.current_release.stable
$candidate = $manifest.releases | Where-Object { $_.hash -eq $stableHash } | Select-Object -First 1
Write-Output ("stable version = {0}  dart = {1}" -f $candidate.version, $candidate.dart_sdk_version)

# Prefer the CN mirror for both metadata and payload.
$mirrors = @(
  'https://storage.flutter-io.cn/flutter_infra_release/releases/',
  'https://mirrors.tuna.tsinghua.edu.cn/flutter/flutter_infra_release/releases/',
  'https://storage.googleapis.com/flutter_infra_release/releases/'
)

$expected = 0
$chosen = $null
foreach ($m in $mirrors) {
  $u = $m + $candidate.archive
  try {
    $head = & $curl -sIL --max-time 30 $u 2>$null | Select-String -Pattern '^content-length:\s*(\d+)' | Select-Object -Last 1
    if ($head) {
      $expected = [int64]$head.Matches[0].Groups[1].Value
      $chosen = $u
      Write-Output ("mirror OK  {0}  size={1} MB" -f $m, [math]::Round($expected/1MB,1))
      break
    }
  } catch { }
}
if (-not $chosen) { throw "no mirror returned content-length for $($candidate.archive)" }

$attempt = 0
$sw = [System.Diagnostics.Stopwatch]::StartNew()
while ($true) {
  $local = if (Test-Path $zipPath) { (Get-Item $zipPath).Length } else { 0 }
  if ($expected -gt 0 -and $local -ge $expected) { break }
  $attempt++
  if ($attempt -gt 400) { throw "gave up after $attempt attempts at $([math]::Round($local/1MB,1)) MB / $([math]::Round($expected/1MB,1)) MB" }
  Write-Output ("[attempt {0}] at {1} MB / {2} MB" -f $attempt, [math]::Round($local/1MB,1), [math]::Round($expected/1MB,1))
  & $curl -sL --fail --max-time 180 -C - --output $zipPath $chosen 2>$null
  $now = if (Test-Path $zipPath) { (Get-Item $zipPath).Length } else { 0 }
  Write-Output ("   -> now {0} MB (gained {1} MB)" -f [math]::Round($now/1MB,1), [math]::Round(($now-$local)/1MB,1))
  if ($now -le $local) { Start-Sleep -Seconds 5 }
}
$sw.Stop()
Write-Output ("DOWNLOAD COMPLETE {0} MB in {1}s" -f [math]::Round((Get-Item $zipPath).Length/1MB,1), [int]$sw.Elapsed.TotalSeconds)

Write-Output '=== extracting ==='
if (-not (Test-Path (Join-Path $flutterDir 'bin\flutter.bat'))) {
  Expand-Archive -Path $zipPath -DestinationPath $installRoot -Force
}
if (-not (Test-Path (Join-Path $flutterDir 'bin\flutter.bat'))) { throw "flutter.bat missing after extraction" }

$env:PATH = (Join-Path $flutterDir 'bin') + ';' + $env:PATH
Write-Output '=== flutter --version ==='
& (Join-Path $flutterDir 'bin\flutter.bat') --version 2>&1 | ForEach-Object { Write-Output $_ }
Write-Output '=== FLUTTER_READY ==='
