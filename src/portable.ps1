$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$archive = Join-Path $env:FERRITE_STAGE 'download.tmp'
Invoke-WebRequest -UseBasicParsing -Uri $env:FERRITE_URL -OutFile $archive -TimeoutSec 240
if ((Get-FileHash -LiteralPath $archive).Hash -ne $env:FERRITE_HASH) {
    throw 'Empreinte SHA-256 du téléchargement incorrecte.'
}
$names = $env:FERRITE_FILES.Split(',')
if ($env:FERRITE_URL.EndsWith('.zip')) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        foreach ($name in $names) {
            $entries = @($zip.Entries | Where-Object Name -eq $name)
            if ($entries.Count -ne 1) { throw "Exécutable introuvable ou ambigu : $name" }
            $output = Join-Path $env:FERRITE_STAGE $name
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $output, $false)
        }
    } finally { $zip.Dispose() }
    [IO.File]::Delete($archive)
} else {
    [IO.File]::Move($archive, (Join-Path $env:FERRITE_STAGE $names[0]))
}
