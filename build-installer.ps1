# Builds aio-screen in release mode and packages it with Inno Setup into dist\.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

$version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value

if (-not (Test-Path assets\icon.ico)) {
    cargo build --release
    if ($LASTEXITCODE) { exit $LASTEXITCODE }
    Start-Process -FilePath target\release\aio-screen.exe -ArgumentList 'make-icon', 'assets' -Wait -WorkingDirectory $PSScriptRoot
}
# Built after the icon exists so the exe embeds it.
cargo build --release
if ($LASTEXITCODE) { exit $LASTEXITCODE }

$iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    $cmd = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($cmd) { $iscc = $cmd.Source }
}
if (-not $iscc) {
    Write-Error "Inno Setup 6 not found. Install it from https://jrsoftware.org/isdl.php (or: winget install JRSoftware.InnoSetup), then rerun."
}

& $iscc "/DAppVersion=$version" installer\aio-screen.iss
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Get-ChildItem dist\*.exe | Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 1) } }
