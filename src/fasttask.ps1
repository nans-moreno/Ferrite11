param([ValidateSet('replace','restore')][string]$Action = $env:FERRITE_TASK_ACTION)
function Message([string]$fr, [string]$en) {
    if ($env:FERRITE_LANGUAGE -eq 'en') { return $en }
    return $fr
}
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$directory = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'FastTask'
$exe = Join-Path $directory 'FastTask.exe'
$key = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options\taskmgr.exe'
function CurrentManager {
    if (Test-Path -LiteralPath $key) {
        return (Get-Item -LiteralPath $key).GetValue('Debugger', '')
    }
    return ''
}
function CheckManager {
    $value = ([string](CurrentManager)).Trim().Trim('"')
    if ($value -and $value -ne $exe) { throw (Message "Un autre remplacement est actif. Retablis-le depuis son application avant de continuer." "Another replacement is active. Restore the default in that app first.") }
    return ($value -eq $exe)
}
$restore = $Action -eq 'restore'
if ((CheckManager) -eq (!$restore)) { return }
if (!(Test-Path -LiteralPath $exe -PathType Leaf)) {
    if ($restore) { throw (Message "FastTask est introuvable. Reinstalle la version 1.1.0 dans Program Files pour restaurer le gestionnaire Windows." "FastTask was not found. Reinstall version 1.1.0 in Program Files to restore Windows Task Manager.") }
    $stage = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid().ToString('N'))
    [IO.Directory]::CreateDirectory($stage) | Out-Null
    $setup = Join-Path $stage 'FastTask-Setup-1.1.0.exe'
    try {
        Invoke-WebRequest -UseBasicParsing -Uri 'https://github.com/ilyafainberg/FastTask/releases/download/v1.1.0/FastTask-Setup-1.1.0.exe' -OutFile $setup -TimeoutSec 180
        $locked = [IO.File]::OpenRead($setup)
        try {
            if ((Get-FileHash -InputStream $locked).Hash -ne '9f8693c1445a895f4b78e2f806a94246302e7b91d6c303f29b22abed6da14ff0') { throw (Message "Installateur non conforme." "The installer failed verification.") }
            $process = Start-Process -FilePath $setup -Verb RunAs -ArgumentList @('/ALLUSERS', '/SILENT', '/NORESTART', ('/DIR="' + $directory + '"')) -Wait -PassThru
            if ($process.ExitCode -ne 0) { throw (Message "Installation interrompue ($($process.ExitCode))." "Installation interrupted ($($process.ExitCode)).") }
        } finally { $locked.Dispose() }
    } finally {
        if ([IO.File]::Exists($setup)) { [IO.File]::Delete($setup) }
        [IO.Directory]::Delete($stage)
    }
}
$checked = [IO.File]::OpenRead($exe)
try {
    if ((Get-FileHash -InputStream $checked).Hash -ne 'D3CCB5CE3ED625DB4662A972DC367BB7900AC49648D6A23C29FB3F07B9A3C63F') {
        throw (Message "Cette version de FastTask differe de la version verifiee. Utilise son bouton Set as Default ou Restore Task Manager." "This FastTask version differs from the verified release. Use its Set as Default or Restore Task Manager button.")
    }
    if ((CheckManager) -eq (!$restore)) { return }
    $flag = if ($restore) { '--unregister' } else { '--register' }
    $process = Start-Process -FilePath $exe -Verb RunAs -ArgumentList $flag -Wait -PassThru
    if ($process.ExitCode -ne 0 -or (CheckManager) -eq $restore) { throw (Message "Windows ne confirme pas le changement." "Windows could not confirm the change.") }
} finally { $checked.Dispose() }
