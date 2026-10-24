#!/usr/bin/env pwsh
# check-fixture-set.ps1 — prove the local fixture set actually wires the mock
# and the product together.
#
# The set in this directory is only useful if a launcher can start the stack
# from it. This script does exactly that, over HTTP, with the values the
# fixture set carries:
#
#   1. reads local.env the way a launcher reads it;
#   2. starts `mock-line-identity` with the fixture's LINE_ID_TOKEN_KEY and
#      LINE_CHANNEL_ID;
#   3. starts `commerce-identity` with the same two values;
#   4. mints a token at the mock and posts it to the product's
#      `/v1/sessions/line` — the product must accept it;
#   5. mints each broken flavour and posts it too — the product must refuse
#      it with its own problem code.
#
# Usage:  pwsh -File tools/commerce-e2e/mock_data/local/check-fixture-set.ps1
#
# The containers it starts are removed before it exits.
#
# Note: the identity service registers `/healthz` with POST only
# (`commerce-identity/src/lib.rs:320`), so its readiness probe below is a
# POST — a GET answers 405 while the service is up and well.

$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here '..' '..' '..' '..')

# The fixture set, read the way a launcher reads it.
$envs = @{}
Get-Content (Join-Path $here 'local.env') |
    Where-Object { $_ -match '^[A-Z_]+=' } |
    ForEach-Object { $k, $v = $_ -split '=', 2; $envs[$k] = $v }

$key = $envs['LINE_ID_TOKEN_KEY']
$chan = $envs['LINE_CHANNEL_ID']
Write-Output "fixture: LINE_ID_TOKEN_KEY=$key LINE_CHANNEL_ID=$chan"

$volume = "$($root.Path)".Replace('\', '/')
$mockName = 'dokipon-fx-mock'
$idName = 'dokipon-fx-identity'

docker rm -f $mockName $idName 2>&1 | Out-Null

docker run -d --name $mockName -p 9004:9004 `
    -v "${volume}:/work" -v dokipon-cargo-home:/cargo `
    -e CARGO_HOME=/cargo -e PORT=9004 -e LINE_ID_TOKEN_KEY=$key -e LINE_CHANNEL_ID=$chan `
    -w /work/services rust:1.98.1-slim-bookworm `
    cargo run -p commerce-mocks --bin mock-line-identity | Out-Null

docker run -d --name $idName -p 8081:8081 `
    -v "${volume}:/work" -v dokipon-cargo-home:/cargo `
    -e CARGO_HOME=/cargo -e PORT=8081 -e LINE_ID_TOKEN_KEY=$key -e LINE_CHANNEL_ID=$chan `
    -e DOKIPON_ALLOW_DEV_SECRETS=1 `
    -w /work/services rust:1.98.1-slim-bookworm `
    cargo run -p commerce-identity | Out-Null

function Wait-For($uri, $method) {
    foreach ($i in 1..60) {
        try {
            Invoke-WebRequest -Uri $uri -Method $method -TimeoutSec 5 -UseBasicParsing | Out-Null
            return $true
        } catch { Start-Sleep -Seconds 2 }
    }
    return $false
}

$mockUp = Wait-For 'http://127.0.0.1:9004/healthz' 'GET'
$idUp = Wait-For 'http://127.0.0.1:8081/healthz' 'POST'
Write-Output "probe mock /healthz (GET)  -> $mockUp"
Write-Output "probe identity /healthz (POST) -> $idUp"
if (-not ($mockUp -and $idUp)) {
    docker logs $mockName
    docker logs $idName
    docker rm -f $mockName $idName 2>&1 | Out-Null
    exit 1
}

function Mint($flavour) {
    $u = 'http://127.0.0.1:9004/mockline/mint'
    if ($flavour) { $u += "?flavour=$flavour" }
    return (Invoke-RestMethod -Uri $u -TimeoutSec 10).idToken
}

function Post($token) {
    try {
        $r = Invoke-WebRequest -Uri 'http://127.0.0.1:8081/v1/sessions/line' -Method POST `
            -ContentType 'application/json' -Body (@{ id_token = $token } | ConvertTo-Json) `
            -TimeoutSec 10 -UseBasicParsing
        return @{ status = [int]$r.StatusCode; body = $r.Content }
    } catch {
        return @{ status = [int]$_.Exception.Response.StatusCode; body = $_.ErrorDetails.Message }
    }
}

$ok = Post (Mint '')
Write-Output "valid token         -> HTTP $($ok.status) $($ok.body)"

$expected = @{
    'wrong_audience'  = 'identity.bad_audience'
    'wrong_issuer'    = 'identity.bad_issuer'
    'expired'         = 'identity.expired_token'
    'wrong_signature' = 'identity.bad_signature'
}
$failed = 0
foreach ($flavour in $expected.Keys) {
    $r = Post (Mint $flavour)
    $code = ([regex]::Match($r.body, '"code"\s*:\s*"([^"]+)"')).Groups[1].Value
    $good = ($r.status -eq 401 -and $code -eq $expected[$flavour])
    if (-not $good) { $failed += 1 }
    Write-Output "$flavour -> HTTP $($r.status) code=$code (want $($expected[$flavour]))"
}

docker rm -f $mockName $idName 2>&1 | Out-Null
Write-Output 'containers removed'

if ($ok.status -ne 200 -or $failed -gt 0) { exit 1 }
Write-Output 'fixture set: OK'
