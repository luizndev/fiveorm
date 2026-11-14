# FiveORM installer for Windows x64. No Rust/Cargo/Node.js/npm required.
$ErrorActionPreference = 'Stop'

if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) {
    throw 'FiveORM: Este instalador suporta Windows.'
}
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -ne 'AMD64') {
    throw 'FiveORM: Esta versão suporta Windows x64 (AMD64).'
}

$repo = 'luizndev/fiveorm'
$asset = 'fiveorm-windows-x86_64.zip'
$base = "https://github.com/$repo/releases/latest/download"
$tempDir = Join-Path ([IO.Path]::GetTempPath()) ("fiveorm-" + [guid]::NewGuid().ToString('N'))
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\FiveORM'
$zipPath = Join-Path $tempDir $asset
$checksumsPath = Join-Path $tempDir 'SHA256SUMS'

New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
try {
    Write-Host 'FiveORM: baixando a ultima versao para Windows x64...'
    try {
        Invoke-WebRequest -Uri "$base/$asset" -OutFile $zipPath -ErrorAction Stop
        Invoke-WebRequest -Uri "$base/SHA256SUMS" -OutFile $checksumsPath -ErrorAction Stop
    } catch {
        throw 'FiveORM: download falhou. Verifique se existe uma GitHub Release e se o repositorio e publico.'
    }

    $checksumLine = Get-Content -Path $checksumsPath |
        Where-Object { $_ -match ('^[0-9a-fA-F]{64}\s+' + [regex]::Escape($asset) + '$') } |
        Select-Object -First 1

    if (-not $checksumLine) {
        throw "FiveORM: hash SHA256 ausente para $asset."
    }

    $expected = ($checksumLine -split '\s+')[0].ToUpperInvariant()
    $actual = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToUpperInvariant()
    if ($expected -ne $actual) {
        throw 'FiveORM: verificacao SHA256 falhou. Instalacao cancelada.'
    }

    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
    $binary = Join-Path $tempDir 'fiveorm.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw 'FiveORM: executavel fiveorm.exe ausente no pacote.'
    }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item -LiteralPath $binary -Destination (Join-Path $installDir 'fiveorm.exe') -Force

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @($userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if (-not ($entries | Where-Object { $_.TrimEnd('\') -ieq $installDir.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $installDir) -join ';'), 'User')
    }

    $processEntries = @($env:Path -split ';')
    if (-not ($processEntries | Where-Object { $_.TrimEnd('\') -ieq $installDir.TrimEnd('\') })) {
        $env:Path = "$env:Path;$installDir"
    }

    Write-Host "FiveORM instalado em: $installDir\fiveorm.exe"
    & (Join-Path $installDir 'fiveorm.exe') --version
    Write-Host 'Abra um novo PowerShell e execute: fiveorm init'
}
finally {
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
