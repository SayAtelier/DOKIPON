#requires -Version 7
# Install rustup + toolchain 1.98.1 (pinned by rust-toolchain.toml) into E:\DevTools
# Uses the TUNA mirror because this machine is in CN and static.rust-lang.org is unreliable.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$root = 'E:\DevTools'
$rustupHome = Join-Path $root 'rustup'
$cargoHome  = Join-Path $root 'cargo'
New-Item -ItemType Directory -Path $rustupHome -Force | Out-Null
New-Item -ItemType Directory -Path $cargoHome  -Force | Out-Null

$env:RUSTUP_HOME = $rustupHome
$env:CARGO_HOME  = $cargoHome
$env:RUSTUP_DIST_SERVER = 'https://mirrors.tuna.tsinghua.edu.cn/rustup'
$env:RUSTUP_UPDATE_ROOT = 'https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup'

$curl = "$env:SystemRoot\System32\curl.exe"
$init  = Join-Path $root 'rustup-init.exe'

Write-Output '=== downloading rustup-init.exe ==='
$srcs = @(
  'https://mirrors.tuna.tsinghua.edu.cn/rustup/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe',
  'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
)
$got = $false
foreach ($u in $srcs) {
  & $curl -sL --fail --retry 5 --retry-delay 3 --max-time 300 -o $init $u 2>$null
  if ((Test-Path $init) -and (Get-Item $init).Length -gt 500000) { Write-Output ("got rustup-init from {0} ({1} KB)" -f $u, [int]((Get-Item $init).Length/1KB)); $got = $true; break }
}
if (-not $got) { throw 'could not download rustup-init.exe' }

Write-Output '=== installing rustup (no default toolchain yet) ==='
& $init -y --no-modify-path --default-toolchain none 2>&1 | ForEach-Object { Write-Output $_ }

$rustup = Join-Path $cargoHome 'bin\rustup.exe'
if (-not (Test-Path $rustup)) { throw "rustup.exe not found at $rustup" }

Write-Output '=== installing toolchain 1.98.1 (matches rust-toolchain.toml) ==='
& $rustup toolchain install 1.98.1 --profile default --component clippy,rustfmt 2>&1 | ForEach-Object { Write-Output $_ }
if ($LASTEXITCODE -ne 0) { throw "toolchain install failed ($LASTEXITCODE)" }

& $rustup default 1.98.1 2>&1 | ForEach-Object { Write-Output $_ }

Write-Output '=== versions ==='
& (Join-Path $cargoHome 'bin\cargo.exe') --version
& (Join-Path $cargoHome 'bin\rustc.exe') --version
Write-Output '=== RUST_READY ==='
