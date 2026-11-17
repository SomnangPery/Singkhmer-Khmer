param(
    [switch]$Remove,
    [string]$Dll,
    [string]$InstallDir = (Join-Path $env:ProgramFiles 'Bunny Keyboard'),
    [switch]$AsAdmin,
    [string]$Log
)

$ErrorActionPreference = 'Stop'

$TextService = '{1AF035EE-A180-4349-AD63-A5FC35FE4122}'
# Singkhmer, the one keyboard offered.
$Keyboards = @(
    '{2E577226-7197-4E08-93B6-E5F7DA9937F4}'
)
# Keyboards an earlier version offered (Khmer NiDA): taken off the language
# list by an update.
$RetiredKeyboards = @(
    '{7B70356C-154C-445B-B08A-E771770C5538}'
)
$Tips = $Keyboards | ForEach-Object { "0453:$TextService$_" }
$RetiredTips = $RetiredKeyboards | ForEach-Object { "0453:$TextService$_" }

$DllName = 'khmer_tsf.dll'
$EditorName = 'my-words.ps1'
$StartMenu = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs'
$Shortcut = Join-Path $StartMenu 'Bunny Keyboard - My words.lnk'
# Where version 0.1 installed, under the old name "Khmer Keyboard".
$OldInstallDir = Join-Path $env:ProgramFiles 'Khmer Keyboard'
$OldShortcut = Join-Path $StartMenu 'Khmer Keyboard - My words.lnk'
$Regsvr32 = Join-Path $env:windir 'System32\regsvr32.exe'

function Test-Administrator {
    $me = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
    $me.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Remove-OldCopies {
    if (Test-Path $InstallDir) {
        Get-ChildItem $InstallDir -Filter '*.old' | ForEach-Object {
            try { Remove-Item $_.FullName -Force } catch { }
        }
    }
}

function Clear-InstalledDll {
    $installed = Join-Path $InstallDir $DllName
    if (-not (Test-Path $installed)) { return }
    try {
        Remove-Item $installed -Force
    } catch {
        $aside = Join-Path $InstallDir ('khmer_tsf.{0}.old' -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
        Rename-Item $installed $aside
    }
}

function Invoke-Regsvr32([string[]]$Arguments) {
    $run = Start-Process -FilePath $Regsvr32 -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
    $run.ExitCode
}

function Install-AsAdmin {
    if (-not (Test-Path $Dll)) { throw "There is no file $Dll." }
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    $installed = Join-Path $InstallDir $DllName
    Remove-OldCopies
    Clear-InstalledDll
    Copy-Item $Dll $installed
    $code = Invoke-Regsvr32 @('/s', "`"$installed`"")
    if ($code -ne 0) {
        throw "Windows did not accept the keyboard (regsvr32 gave error $code). Nothing was registered."
    }
    Install-Editor
    Remove-OldName
    "Installed $installed"
}

# The "my words" editor, and a Start menu entry for it. The keyboard finds the
# editor next to the DLL.
function Install-Editor {
    $editor = Join-Path $PSScriptRoot $EditorName
    if (-not (Test-Path $editor)) { return }
    $installed = Join-Path $InstallDir $EditorName
    Copy-Item $editor $installed -Force
    $powershell = Join-Path $env:windir 'System32\WindowsPowerShell\v1.0\powershell.exe'
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($Shortcut)
    $link.TargetPath = $powershell
    $link.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$installed`""
    $link.Description = 'Add your own Singkhmer words to Bunny Keyboard'
    $link.IconLocation = (Join-Path $env:windir 'System32\input.dll') + ',0'
    $link.Save()
    "Added 'Bunny Keyboard - My words' to the Start menu"
}

# An earlier version under the old name "Khmer Keyboard". Windows now loads
# the DLL from the new folder, so the old copy can go (or, if an open app
# still has it in use, be renamed and left for a restart).
function Remove-OldName {
    if (Test-Path $OldShortcut) { Remove-Item $OldShortcut -Force }
    if (-not (Test-Path $OldInstallDir)) { return }
    foreach ($file in @(Get-ChildItem $OldInstallDir -Force)) {
        try {
            Remove-Item $file.FullName -Force
        } catch {
            if ($file.Extension -ne '.old') {
                try { Rename-Item $file.FullName ($file.Name + '.' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.old') } catch { }
            }
        }
    }
    if (-not (Get-ChildItem $OldInstallDir -Force)) { Remove-Item $OldInstallDir -Force }
}

function Remove-Editor {
    if (Test-Path $Shortcut) { Remove-Item $Shortcut -Force }
    $installed = Join-Path $InstallDir $EditorName
    if (Test-Path $installed) { Remove-Item $installed -Force }
}

function Remove-AsAdmin {
    $installed = Join-Path $InstallDir $DllName
    $old = Join-Path $OldInstallDir $DllName
    if ((Test-Path $old) -and -not (Test-Path $installed)) {
        # Only the version under the old name is here: Windows still knows it.
        [void](Invoke-Regsvr32 @('/u', '/s', "`"$old`""))
    }
    if (Test-Path $installed) {
        $code = Invoke-Regsvr32 @('/u', '/s', "`"$installed`"")
        if ($code -ne 0) {
            throw "Windows could not take the keyboard out (regsvr32 gave error $code)."
        }
        Clear-InstalledDll
    }
    Remove-Editor
    Remove-OldName
    Remove-OldCopies
    if ((Test-Path $InstallDir) -and -not (Get-ChildItem $InstallDir -Force)) {
        Remove-Item $InstallDir
        "Removed $InstallDir"
    } elseif (Test-Path $InstallDir) {
        "The keyboard is off this PC. An app that is still open has the old file in use;"
        "it is in $InstallDir and can be deleted after a restart."
    } else {
        "The keyboard was not installed."
    }
}

if ($AsAdmin) {
    try {
        $said = if ($Remove) { Remove-AsAdmin } else { Install-AsAdmin }
        if ($Log) { $said | Set-Content -Path $Log -Encoding UTF8 }
        exit 0
    } catch {
        if ($Log) { "$_" | Set-Content -Path $Log -Encoding UTF8 }
        exit 1
    }
}

function Invoke-AdminPart([string[]]$Arguments) {
    $log = Join-Path $env:TEMP ("khmer-keyboard-{0}.log" -f [Guid]::NewGuid().ToString('N'))
    $all = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-AsAdmin',
        '-InstallDir', "`"$InstallDir`"", '-Log', "`"$log`"") + $Arguments
    $start = @{
        FilePath     = (Get-Process -Id $PID).Path
        ArgumentList = $all
        Wait         = $true
        PassThru     = $true
        WindowStyle  = 'Hidden'
    }
    if (-not (Test-Administrator)) { $start.Verb = 'RunAs' }
    try {
        $run = Start-Process @start
    } catch {
        throw "Permission was not given, so nothing was changed."
    }
    $said = if (Test-Path $log) { Get-Content $log; Remove-Item $log } else { @() }
    if ($run.ExitCode -ne 0) {
        throw (@($said) + "The administrator part failed.")[0]
    }
    $said | ForEach-Object { Write-Host $_ }
}

function Get-BuiltDll {
    $project = Split-Path $PSScriptRoot -Parent
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust is not installed (no 'cargo' command). Get it from https://rustup.rs, or pass -Dll with a built khmer_tsf.dll."
    }
    Write-Host "Building the keyboard..."
    Push-Location $project
    try {
        & cargo build --release -p khmer-windows
        if ($LASTEXITCODE -ne 0) { throw "The build failed." }
        $metadata = & cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
    } finally {
        Pop-Location
    }
    $built = Join-Path $metadata.target_directory "release\$DllName"
    if (-not (Test-Path $built)) { throw "The build did not make $built." }
    $built
}

function Set-Keyboards([bool]$Present) {
    $languages = Get-WinUserLanguageList
    $khmer = $languages | Where-Object { $_.LanguageTag -eq 'km' -or $_.LanguageTag -like 'km-*' } | Select-Object -First 1
    $changed = @()
    if (-not $khmer) {
        if (-not $Present) { return @() }
        $languages.Add('km')
        $khmer = $languages[$languages.Count - 1]
        # Windows adds its own Khmer keyboard with the language. This PC did
        # not have Khmer before, so Bunny Keyboard is the only one it needs.
        foreach ($tip in @($khmer.InputMethodTips)) {
            [void]$khmer.InputMethodTips.Remove($tip)
        }
        $changed += 'km'
    }
    foreach ($tip in $RetiredTips) {
        if ($khmer.InputMethodTips -contains $tip) {
            [void]$khmer.InputMethodTips.Remove($tip)
            $changed += $tip
        }
    }
    foreach ($tip in $Tips) {
        $has = $khmer.InputMethodTips -contains $tip
        if ($Present -and -not $has) {
            $khmer.InputMethodTips.Add($tip)
            $changed += $tip
        } elseif (-not $Present -and $has) {
            [void]$khmer.InputMethodTips.Remove($tip)
            $changed += $tip
        }
    }
    # Removing the last Khmer keyboard: take the language off too, or Windows
    # would put its own Khmer keyboard back in the list.
    if (-not $Present -and $changed -and $khmer.InputMethodTips.Count -eq 0) {
        [void]$languages.Remove($khmer)
    }
    if ($changed) {
        Set-WinUserLanguageList $languages -Force -WarningAction SilentlyContinue
    }
    $changed
}

try {
    if ($Remove) {
        [void](Set-Keyboards $false)
        Invoke-AdminPart @('-Remove')
        Write-Host "Bunny Keyboard is removed."
    } else {
        if (-not [Environment]::Is64BitOperatingSystem) { throw "This keyboard is built for 64-bit Windows only." }
        if (-not $Dll) { $Dll = Get-BuiltDll }
        $Dll = (Resolve-Path $Dll).Path
        Invoke-AdminPart @('-Dll', "`"$Dll`"")
        [void](Set-Keyboards $true)
        Write-Host ""
        Write-Host "Bunny Keyboard is installed."
        Write-Host "Press Windows+Space and choose 'Singkhmer (Bunny Keyboard)'."
        Write-Host "Apps that were already open may need to be restarted before they offer it."
    }
} catch {
    Write-Host "Not done: $_" -ForegroundColor Red
    exit 1
}
