# Installs the Khmer keyboard on this PC straight from GitHub, with no Rust
# and no copy of the project. In Windows PowerShell:
#
#   irm https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/setup.ps1 | iex
#
# To remove it again:
#
#   & ([scriptblock]::Create((irm https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download/setup.ps1))) -Remove
#
# This file is run with `iex`, inside the user's own PowerShell window, so it
# never calls `exit` (that would close the window). It only downloads the two
# files and hands over to install.ps1, which runs as a separate program.

param(
    [switch]$Remove,
    # Where the release files are. Only for trying out a different release.
    [string]$From = 'https://github.com/SomnangPery/Singkhmer-Khmer/releases/latest/download'
)

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    try {
        if (-not [Environment]::Is64BitOperatingSystem) { throw "This keyboard is built for 64-bit Windows only." }
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

        function Get-File([string]$Name, [string]$To) {
            try {
                Invoke-WebRequest -UseBasicParsing -Uri "$From/$Name" -OutFile $To
            } catch {
                throw "Could not download $Name from $From. Check the internet connection; if it works, there may be no release yet. ($($_.Exception.Message))"
            }
        }

        $folder = Join-Path $env:TEMP ('khmer-keyboard-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $folder | Out-Null
        try {
            $installer = Join-Path $folder 'install.ps1'
            $dll = Join-Path $folder 'khmer_tsf.dll'
            Write-Host "Downloading the Khmer keyboard..."
            Get-File 'install.ps1' $installer
            $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $installer)
            if ($Remove) {
                $arguments += '-Remove'
            } else {
                Get-File 'khmer_tsf.dll' $dll
                $arguments += @('-Dll', $dll)
            }
            # Windows will ask for permission to change the PC: say Yes.
            & (Join-Path $PSHOME 'powershell.exe') @arguments
        } finally {
            Remove-Item $folder -Recurse -Force -ErrorAction SilentlyContinue
        }
    } catch {
        Write-Host "Not done: $_" -ForegroundColor Red
    }
}
