# Build and package Ferrite11 for Windows x64 using Rust and .NET Framework.
$ErrorActionPreference = 'Stop'
Push-Location -LiteralPath $PSScriptRoot
try {
    $csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
    if (!(Test-Path -LiteralPath $csc)) { throw 'Compilateur C# .NET Framework x64 introuvable.' }
    $toolchain = & rustc -vV
    if ($LASTEXITCODE -ne 0 -or $toolchain -notcontains 'host: x86_64-pc-windows-msvc') {
        throw 'Utilisez une chaine Rust stable x86_64-pc-windows-msvc.'
    }
    & cargo test --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Les tests Rust ont echoue.' }
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'La compilation Rust a echoue.' }
    $metadata = (& cargo metadata --no-deps --locked --format-version 1) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Metadonnees Cargo indisponibles.' }
    $version = ($metadata.packages | Where-Object name -eq 'ferrite11').version
    $dist = Join-Path $PSScriptRoot 'dist'
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    $app = Join-Path $dist 'Ferrite11.exe'
    $setup = Join-Path $dist "Ferrite11-Setup-$version-x64.exe"
    Copy-Item -LiteralPath (Join-Path $metadata.target_directory 'release\ferrite11.exe') -Destination $app
    Copy-Item -LiteralPath 'LICENSE' -Destination (Join-Path $dist 'LICENSE.txt')
    & $csc /nologo /target:winexe /platform:x64 /codepage:65001 /reference:System.Windows.Forms.dll "/resource:$app,Ferrite11.exe" '/resource:LICENSE,LICENSE.txt' "/out:$setup" '.\installer\Setup.cs'
    if ($LASTEXITCODE -ne 0) { throw 'La compilation du programme installateur a echoue.' }
    Get-FileHash -Algorithm SHA256 -LiteralPath $app,$setup | ForEach-Object {
        '{0}  {1}' -f $_.Hash.ToLowerInvariant(),[IO.Path]::GetFileName($_.Path)
    } | Set-Content -LiteralPath (Join-Path $dist 'SHA256.txt') -Encoding ascii
    Write-Host "Compilation terminee dans $dist"
} finally { Pop-Location }
