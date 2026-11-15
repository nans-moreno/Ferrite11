# This helper stays alive across the restart. Only a stable release from this repository is accepted.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$english = $env:FERRITE_UPDATE_LANG -eq 'en'
function Message($fr, $en) { if ($english) { return $en }; return $fr }
function Signal($message) { [Console]::Out.WriteLine($message); [Console]::Out.Flush() }
$stage = $null
$locked = $null
$owner = $null
$closed = $false
$handoff = $false
try {
    $owner = [Diagnostics.Process]::GetProcessById([int]$env:FERRITE_UPDATE_PID)
    $target = $owner.MainModule.FileName
    $folder = [IO.Path]::GetDirectoryName($target)
    $repo = 'nans-moreno/Ferrite11'
    $headers = @{ 'User-Agent' = 'Ferrite11-Updater'; 'Accept' = 'application/vnd.github+json' }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -Headers $headers -TimeoutSec 20
    if ($release.draft -or $release.prerelease -or $release.tag_name -cnotmatch '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$') {
        throw (Message 'Version publiée non reconnue.' 'Unrecognized release version.')
    }
    $version = $release.tag_name.Substring(1)
    if ([version]$version -le [version]$env:FERRITE_UPDATE_VERSION) { Signal 'CURRENT'; return }
    $assetName = "Ferrite11-Setup-$version-x64.exe"
    $asset = @($release.assets | Where-Object { $_.name -ceq $assetName -and $_.state -eq 'uploaded' })
    $url = "https://github.com/$repo/releases/download/v$version/$assetName"
    if ($asset.Count -ne 1 -or $asset[0].browser_download_url -cne $url -or $asset[0].digest -notmatch '^sha256:[a-fA-F0-9]{64}$') {
        throw (Message 'Installateur ou empreinte SHA-256 indisponible pour cette version.' 'The installer or its SHA-256 digest is missing from this release.')
    }
    Signal "DOWNLOAD $version"
    $stage = [IO.Directory]::CreateDirectory([IO.Path]::Combine([IO.Path]::GetTempPath(), 'Ferrite11-' + [Guid]::NewGuid().ToString('N'))).FullName
    $setup = Join-Path $stage $assetName
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $setup -TimeoutSec 180
    $locked = [IO.File]::OpenRead($setup)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $actual = [BitConverter]::ToString($sha.ComputeHash($locked)).Replace('-', '') }
    finally { $sha.Dispose() }
    if ($locked.Length -ne $asset[0].size -or $actual -ne $asset[0].digest.Substring(7)) {
        throw (Message 'Le téléchargement ne correspond pas à la version publiée.' 'The download does not match the published release.')
    }
    # Confirm that a replacement can be staged beside the running executable before asking it to exit.
    $probe = Join-Path $folder ([Guid]::NewGuid().ToString('N') + '.tmp')
    [IO.FileStream]::new($probe, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None, 1, [IO.FileOptions]::DeleteOnClose).Dispose()
    if ($owner.HasExited) { return }
    Signal 'READY'
    $handoff = $true
    if (!$owner.WaitForExit(30000)) { throw (Message 'Ferrite11 est toujours ouvert. Réessaie la mise à jour.' 'Ferrite11 is still running. Please try the update again.') }
    $closed = $true
    $installer = [Diagnostics.ProcessStartInfo]::new()
    $installer.FileName = $setup
    $installer.Arguments = '/update-file "' + $target + '"'
    $installer.UseShellExecute = $false
    $install = [Diagnostics.Process]::Start($installer)
    $install.WaitForExit()
    if ($install.ExitCode -ne 0) { throw (Message 'Installation interrompue. Vérifie les droits du dossier.' 'Installation failed. Check that you can write to the installation folder.') }
    Start-Process -FilePath $target -WorkingDirectory $folder
    $closed = $false
} catch {
    $detail = $_.Exception.Message -replace '[\r\n]+',' '
    if ($handoff) {
        Add-Type -AssemblyName System.Windows.Forms
        [void][Windows.Forms.MessageBox]::Show($detail, 'Ferrite11')
        if ($closed -and [IO.File]::Exists($target)) { Start-Process -FilePath $target -WorkingDirectory $folder }
    } else { Signal "ERROR $detail" }
} finally {
    if ($locked) { $locked.Dispose() }
    if ($owner) { $owner.Dispose() }
    if ($stage) {
        if ([IO.File]::Exists($setup)) { [IO.File]::Delete($setup) }
        [IO.Directory]::Delete($stage)
    }
}
