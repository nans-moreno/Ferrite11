function Message([string]$fr, [string]$en) {
    if ($env:FERRITE_LANGUAGE -eq 'en') { return $en }
    return $fr
}
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$archive = Join-Path $env:FERRITE_STAGE 'download.tmp'
Invoke-WebRequest -UseBasicParsing -Uri $env:FERRITE_URL -OutFile $archive -TimeoutSec 240
if ((Get-FileHash -LiteralPath $archive).Hash -ne $env:FERRITE_HASH) {
    throw (Message "Empreinte SHA-256 incorrecte." "SHA-256 verification failed.")
}
$names = $env:FERRITE_FILES.Split(',')
if ($env:FERRITE_URL.EndsWith('.zip')) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($archive)
    try {
        foreach ($name in $names) {
            $entries = @($zip.Entries | Where-Object {
                $_.FullName -eq $name -or $_.FullName.EndsWith('/' + $name, [StringComparison]::OrdinalIgnoreCase)
            })
            if ($entries.Count -ne 1) { throw (Message "Fichier introuvable ou ambigu : $name" "Missing or ambiguous file: $name") }
            $output = Join-Path $env:FERRITE_STAGE $name
            [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($output)) | Out-Null
            [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $output, $false)
        }
    } finally { $zip.Dispose() }
    [IO.File]::Delete($archive)
} else {
    [IO.File]::Move($archive, (Join-Path $env:FERRITE_STAGE $names[0]))
}
