param(
    [switch]$Remove,
    [string]$Dll,
    [string]$InstallDir = (Join-Path $env:ProgramFiles 'Khmer Keyboard'),
    [switch]$AsAdmin,
    [string]$Log
)

$ErrorActionPreference = 'Stop'

$TextService = '{1AF035EE-A180-4349-AD63-A5FC35FE4122}'
$Keyboards = @(
    '{2E577226-7197-4E08-93B6-E5F7DA9937F4}'
    '{7B70356C-154C-445B-B08A-E771770C5538}'
)
$Tips = $Keyboards | ForEach-Object { "0453:$TextService$_" }

$DllName = 'khmer_tsf.dll'
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
    "Installed $installed"
}

function Remove-AsAdmin {
    $installed = Join-Path $InstallDir $DllName
    if (Test-Path $installed) {
        $code = Invoke-Regsvr32 @('/u', '/s', "`"$installed`"")
        if ($code -ne 0) {
            throw "Windows could not take the keyboard out (regsvr32 gave error $code)."
        }
        Clear-InstalledDll
    }
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
    if (-not $khmer) {
        if (-not $Present) { return @() }
        $languages.Add('km')
        $khmer = $languages[$languages.Count - 1]
    }
    $changed = @()
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
    if ($changed) {
        Set-WinUserLanguageList $languages -Force -WarningAction SilentlyContinue
    }
    $changed
}

try {
    if ($Remove) {
        [void](Set-Keyboards $false)
        Invoke-AdminPart @('-Remove')
        Write-Host "The Khmer keyboard is removed."
    } else {
        if (-not [Environment]::Is64BitOperatingSystem) { throw "This keyboard is built for 64-bit Windows only." }
        if (-not $Dll) { $Dll = Get-BuiltDll }
        $Dll = (Resolve-Path $Dll).Path
        Invoke-AdminPart @('-Dll', "`"$Dll`"")
        [void](Set-Keyboards $true)
        Write-Host ""
        Write-Host "The Khmer keyboard is installed."
        Write-Host "Press Windows+Space and choose 'Singkhmer (Khmer Keyboard)' or 'Khmer NiDA (Khmer Keyboard)'."
        Write-Host "Apps that were already open may need to be restarted before they offer it."
    }
} catch {
    Write-Host "Not done: $_" -ForegroundColor Red
    exit 1
}
