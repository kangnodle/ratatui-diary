#Requires -Version 5.1
[CmdletBinding()]
param(
    [switch] $SkipTest,
    [switch] $SkipBuild,
    [ValidatePattern('\A[0-9A-Za-z][0-9A-Za-z.+-]*\z')]
    [string] $Version,
    [ValidatePattern('\A[A-Za-z0-9_.-]+\z')]
    [string] $Target,
    [string] $OutDir,
    [switch] $InstallInno
)

$ErrorActionPreference = 'Stop'

$AppId        = '8F3A2C41-9D6E-4B77-A1C5-2E0B7D4F6A83'
$AppName      = 'Ratatui Diary'
$AppPublisher = 'mingyuchoo'
$AppUrl       = 'https://github.com/mingyuchoo/study-series-for-rust'
$BinName      = 'ratatui-diary'
$PackageName  = 'ratatui-diary'

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $ProjectRoot

function Write-Step {
    param([Parameter(Mandatory)] [string] $Message)
    Write-Host ""
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Done {
    param([Parameter(Mandatory)] [string] $Message)
    Write-Host "==> $Message" -ForegroundColor Green
}

function Invoke-Native {
    param(
        [Parameter(Mandatory)] [string]   $Name,
        [Parameter(Mandatory)] [string]   $FilePath,
        [Parameter(Mandatory)] [string[]] $Arguments
    )

    Write-Step "$Name : $FilePath $($Arguments -join ' ')"
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Name 단계 실패 (exit code: $LASTEXITCODE)"
    }
    Write-Done "$Name 완료"
}

function Find-Iscc {
    $cmd = Get-Command 'ISCC.exe' -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }

    $candidates = @(
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
        "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
        "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
    )
    foreach ($path in $candidates) {
        if ($path -and (Test-Path -LiteralPath $path)) { return $path }
    }

    $keys = @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1',
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1',
        'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Inno Setup 6_is1'
    )
    foreach ($key in $keys) {
        try {
            $loc = (Get-ItemProperty -Path $key -Name 'InstallLocation' -ErrorAction Stop).InstallLocation
        }
        catch { continue }
        if ($loc) {
            $exe = Join-Path $loc 'ISCC.exe'
            if (Test-Path -LiteralPath $exe) { return $exe }
        }
    }

    return $null
}

function Get-ArchInfo {
    param([Parameter(Mandatory)] [string] $Triple)

    switch -Regex ($Triple) {
        '^aarch64-' { return @{ Label = 'arm64'; Inno = 'arm64' } }
        '^x86_64-'  { return @{ Label = 'x64';   Inno = 'x64compatible' } }
        '^i686-'    { return @{ Label = 'x86';   Inno = 'x86compatible' } }
        default     { throw "지원하지 않는 대상 트리플입니다: $Triple" }
    }
}

try {
    foreach ($tool in @('cargo', 'rustc')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "$tool 를 찾을 수 없습니다. Rust 툴체인을 설치하세요: https://rustup.rs"
        }
    }

    $hostTriple = (& rustc -vV | Select-String -Pattern '^host:\s*(.+)$').Matches[0].Groups[1].Value.Trim()
    if (-not $Target) { $Target = $hostTriple }
    $arch = Get-ArchInfo -Triple $Target

    if (-not $Version) {
        $metaJson = & cargo metadata --no-deps --format-version 1
        if ($LASTEXITCODE -ne 0) { throw "cargo metadata 실행 실패 (exit code: $LASTEXITCODE)" }
        $meta = $metaJson | ConvertFrom-Json
        $pkg = $meta.packages | Where-Object { $_.name -eq $PackageName }
        if (-not $pkg) { throw "워크스페이스에서 '$PackageName' 패키지를 찾을 수 없습니다." }
        $Version = $pkg.version
        $targetDir = $meta.target_directory
    }
    else {
        $metaJson = & cargo metadata --no-deps --format-version 1
        if ($LASTEXITCODE -ne 0) { throw "cargo metadata 실행 실패 (exit code: $LASTEXITCODE)" }
        $targetDir = ($metaJson | ConvertFrom-Json).target_directory
    }

    $numericVersion = ([regex]::Match($Version, '^\d+(\.\d+){0,3}')).Value
    if (-not $numericVersion) { $numericVersion = '0.0.0' }

    if (-not $OutDir) { $OutDir = Join-Path $ProjectRoot 'dist' }
    $OutDir     = [System.IO.Path]::GetFullPath($OutDir)
    $stagingDir = Join-Path $OutDir ("staging-" + [guid]::NewGuid().ToString('N'))
    $issPath    = Join-Path $OutDir "$PackageName.iss"
    $outBase    = "$PackageName-$Version-$($arch.Label)-setup"
    $setupPath  = Join-Path $OutDir "$outBase.exe"

    Write-Host ""
    Write-Host "  패키지 : $PackageName $Version" -ForegroundColor White
    Write-Host "  대상   : $Target ($($arch.Label))" -ForegroundColor White
    Write-Host "  산출물 : $setupPath" -ForegroundColor White

    if (-not $SkipTest) {
        Write-Step 'Verify : cargo fmt --all -- --check'
        & cargo fmt --all -- --check
        if ($LASTEXITCODE -ne 0) {
            throw "포맷이 맞지 않습니다. 'cargo fmt --all' 실행 후 다시 시도하세요."
        }
        Write-Done 'Format 확인 완료'

        Invoke-Native -Name 'Clippy' -FilePath 'cargo' -Arguments @('clippy', '--workspace', '--all-targets')
        Invoke-Native -Name 'Test'   -FilePath 'cargo' -Arguments @('test', '--workspace')
    }
    else {
        Write-Host ""
        Write-Host "==> Verify 단계 건너뜀 (-SkipTest)" -ForegroundColor Yellow
    }

    $useExplicitTarget = ($Target -ne $hostTriple) -or $PSBoundParameters.ContainsKey('Target')
    if ($useExplicitTarget) {
        $binPath = Join-Path $targetDir "$Target\release\$BinName.exe"
    }
    else {
        $binPath = Join-Path $targetDir "release\$BinName.exe"
    }

    if (-not $SkipBuild) {
        $buildArgs = @('build', '--release', '-p', $PackageName)
        if ($useExplicitTarget) { $buildArgs += @('--target', $Target) }
        Invoke-Native -Name 'Build' -FilePath 'cargo' -Arguments $buildArgs
    }
    else {
        Write-Host ""
        Write-Host "==> Build 단계 건너뜀 (-SkipBuild)" -ForegroundColor Yellow
    }

    if (-not (Test-Path -LiteralPath $binPath)) {
        throw "릴리스 바이너리를 찾을 수 없습니다: $binPath"
    }

    Write-Step "Stage : $stagingDir"
    New-Item -ItemType Directory -Path $stagingDir | Out-Null

    Copy-Item -LiteralPath $binPath -Destination $stagingDir -Force
    foreach ($doc in @('README.md', 'LICENSE', 'LICENSE.md', 'LICENSE.txt')) {
        $docPath = Join-Path $ProjectRoot $doc
        if (Test-Path -LiteralPath $docPath) {
            Copy-Item -LiteralPath $docPath -Destination $stagingDir -Force
        }
    }
    $stagedFiles = @(Get-ChildItem -LiteralPath $stagingDir -File)
    $filesSection = ($stagedFiles | ForEach-Object {
        Write-Host ("    + {0} ({1:N0} bytes)" -f $_.Name, $_.Length) -ForegroundColor DarkGray
        'Source: "{0}"; DestDir: "{{app}}"; Flags: ignoreversion' -f $_.Name
    }) -join "`r`n"
    Write-Done 'Stage 완료'

    $iss = @"
[Setup]
AppId={{$AppId}
AppName=$AppName
AppVersion=$Version
VersionInfoVersion=$numericVersion
AppPublisher=$AppPublisher
AppPublisherURL=$AppUrl
AppSupportURL=$AppUrl
DefaultDirName={autopf}\$AppName
DefaultGroupName=$AppName
UninstallDisplayName=$AppName $Version
UninstallDisplayIcon={app}\$BinName.exe
OutputDir=$OutDir
OutputBaseFilename=$outBase
SourceDir=$stagingDir
PrivilegesRequired=lowest
ArchitecturesAllowed=$($arch.Inno)
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
AllowNoIcons=yes
DisableProgramGroupPage=yes
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "addtopath";  Description: "PATH에 추가 (터미널에서 $BinName 명령으로 실행)"; GroupDescription: "추가 작업:"
Name: "startmenu";  Description: "시작 메뉴에 바로가기 만들기";                    GroupDescription: "추가 작업:"

[Files]
$filesSection

[Icons]
Name: "{autoprograms}\$AppName"; Filename: "{app}\$BinName.exe"; Tasks: startmenu

[Code]
const
  EnvironmentKey = 'Environment';

procedure EnvAddPath(InstlPath: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    Paths := '';

  if Pos(';' + Uppercase(InstlPath) + ';', ';' + Uppercase(Paths) + ';') > 0 then
    exit;

  if Paths = '' then
    Paths := InstlPath
  else if Paths[Length(Paths)] = ';' then
    Paths := Paths + InstlPath
  else
    Paths := Paths + ';' + InstlPath;

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure EnvRemovePath(InstlPath: string);
var
  Paths: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    exit;

  P := Pos(';' + Uppercase(InstlPath) + ';', ';' + Uppercase(Paths) + ';');
  if P = 0 then
    exit;

  if P = 1 then
    Delete(Paths, 1, Length(InstlPath) + 1)
  else
    Delete(Paths, P - 1, Length(InstlPath) + 1);

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    EnvAddPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    EnvRemovePath(ExpandConstant('{app}'));
end;
"@

    Write-Step "Generate : $issPath"
    [System.IO.File]::WriteAllText($issPath, $iss, (New-Object System.Text.UTF8Encoding($true)))
    Write-Done 'Generate 완료'

    $iscc = Find-Iscc
    if (-not $iscc -and $InstallInno) {
        if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
            throw 'winget을 찾을 수 없어 Inno Setup을 자동 설치할 수 없습니다.'
        }
        Invoke-Native -Name 'Install Inno Setup' -FilePath 'winget' -Arguments @(
            'install', '--id', 'JRSoftware.InnoSetup', '--exact', '--silent',
            '--accept-package-agreements', '--accept-source-agreements'
        )
        $iscc = Find-Iscc
    }
    if (-not $iscc) {
        throw @'
Inno Setup 컴파일러(ISCC.exe)를 찾을 수 없습니다. 아래 중 하나를 선택하세요.
  - 이 스크립트에 -InstallInno 스위치를 붙여 다시 실행
  - 직접 설치: winget install --id JRSoftware.InnoSetup --exact
  - 수동 설치: https://jrsoftware.org/isdl.php  (Inno Setup 6.3 이상)
'@
    }
    Write-Host ""
    Write-Host "  ISCC   : $iscc" -ForegroundColor White

    Invoke-Native -Name 'Package' -FilePath $iscc -Arguments @($issPath)

    if (-not (Test-Path -LiteralPath $setupPath)) {
        throw "설치 파일이 생성되지 않았습니다: $setupPath"
    }
    $setupSize = (Get-Item -LiteralPath $setupPath).Length

    Write-Host ""
    Write-Host "설치 파일 생성 완료" -ForegroundColor Green
    Write-Host "  $setupPath" -ForegroundColor White
    Write-Host ("  {0:N2} MB" -f ($setupSize / 1MB)) -ForegroundColor White
    Write-Host ""
    Write-Host "설치하려면 위 파일을 실행하세요. PATH를 선택했다면 새 터미널에서 '$BinName' 명령을 쓸 수 있습니다." -ForegroundColor DarkGray
}
catch {
    Write-Host ""
    Write-Host "!! $($_.Exception.Message)" -ForegroundColor Red
    Pop-Location
    exit 1
}

Pop-Location
