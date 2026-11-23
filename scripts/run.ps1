#Requires -Version 5.1
[CmdletBinding()]
param(
    [switch] $Release,
    [switch] $SkipTest,
    [switch] $NoRun
)

$ErrorActionPreference = 'Stop'

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $ProjectRoot

function Invoke-Step {
    param(
        [Parameter(Mandatory)] [string]   $Name,
        [Parameter(Mandatory)] [string[]] $CargoArgs
    )

    Write-Host ""
    Write-Host "==> $Name : cargo $($CargoArgs -join ' ')" -ForegroundColor Cyan

    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "!! $Name 단계 실패 (exit code: $LASTEXITCODE)" -ForegroundColor Red
        Pop-Location
        exit $LASTEXITCODE
    }
    Write-Host "==> $Name 완료" -ForegroundColor Green
}

try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "!! cargo를 찾을 수 없습니다. Rust 툴체인을 설치하세요: https://rustup.rs" -ForegroundColor Red
        exit 1
    }

    $profileArgs = if ($Release) { @('--release') } else { @() }

    Invoke-Step -Name 'Build' -CargoArgs (@('build', '--workspace') + $profileArgs)

    if (-not $SkipTest) {
        Invoke-Step -Name 'Test' -CargoArgs (@('test', '--workspace') + $profileArgs)
    }
    else {
        Write-Host ""
        Write-Host "==> Test 단계 건너뜀 (-SkipTest)" -ForegroundColor Yellow
    }

    if (-not $NoRun) {
        Invoke-Step -Name 'Run' -CargoArgs (@('run') + $profileArgs)
    }
    else {
        Write-Host ""
        Write-Host "==> Run 단계 건너뜀 (-NoRun)" -ForegroundColor Yellow
    }
}
finally {
    Pop-Location
}
