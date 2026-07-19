<#
Headless WinDbg (cdb.exe) triage for a League of Legends crash dump.
Auto-finds the newest large .tmp in %TEMP% (LoL's "Crash Dump" dialog writes
a full-process dump there) if -DumpPath isn't given, copies it out of %TEMP%
so dismissing the dialog can't delete it mid-analysis, then runs cdb and
prints just the sections that matter (exception, faulting module, stack,
!analyze bucket) instead of the raw 100k+ token transcript.

Usage:
  scripts\analyze-league-crash.ps1                      # auto-find newest dump
  scripts\analyze-league-crash.ps1 -DumpPath C:\...\x.tmp
  scripts\analyze-league-crash.ps1 -CheckOnly            # just verify cdb.exe resolves
#>
param(
    [string]$DumpPath,
    [string]$SymbolPath = "D:\.cargo\target\x86_64-pc-windows-msvc\release;srv*C:\symcache*https://msdl.microsoft.com/download/symbols",
    [string]$OutDir = "$env:TEMP\WildSkin-crash-analysis",
    [switch]$CheckOnly
)
$ErrorActionPreference = 'Stop'

function Resolve-Cdb {
    $pkg = Get-AppxPackage -Name 'Microsoft.WinDbg' -ErrorAction SilentlyContinue
    if (-not $pkg) {
        throw "WinDbg (Microsoft.WinDbg AppX package) isn't installed — get it from the Microsoft Store."
    }
    $cdb = Join-Path $pkg.InstallLocation 'amd64\cdb.exe'
    if (-not (Test-Path $cdb)) {
        throw "Found the WinDbg package but not cdb.exe at expected path: $cdb"
    }
    return $cdb
}

$cdb = Resolve-Cdb
if ($CheckOnly) {
    "cdb.exe found: $cdb"
    (Get-Item $cdb).VersionInfo.FileVersion
    return
}

if (-not $DumpPath) {
    # ponytail: size-floor heuristic (>50MB), not a real dump-file signature check —
    # good enough because LoL's full-process dumps are multi-GB and %TEMP% otherwise
    # only has small scratch files; tighten if that ever collides.
    $candidate = Get-ChildItem -Path $env:TEMP -Filter *.tmp -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Length -gt 50MB } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $candidate) {
        throw "No dump auto-found in `$env:TEMP (looked for a *.tmp over 50MB). " +
              "If the 'Crash Dump' dialog is still open, pass its path via -DumpPath before dismissing it — the dialog deletes the file on close."
    }
    $DumpPath = $candidate.FullName
}
if (-not (Test-Path -LiteralPath $DumpPath)) {
    throw "Dump not found: $DumpPath"
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$dumpCopy = Join-Path $OutDir "$stamp.dmp"
$logPath = Join-Path $OutDir "$stamp.log"
Copy-Item -LiteralPath $DumpPath -Destination $dumpCopy -Force

$cmds = '.ecxr; r; kb 40; !analyze -v; lm; q'
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $cdb
$psi.Arguments = '-z "' + $dumpCopy + '" -c "' + $cmds + '"'
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
# _NT_SYMBOL_PATH via env var, not `.sympath` in -c: a `.sympath` value passed
# inline in -c gets glued to the rest of the semicolon-separated command string.
$psi.EnvironmentVariables['_NT_SYMBOL_PATH'] = $SymbolPath

$proc = [System.Diagnostics.Process]::Start($psi)
$out = $proc.StandardOutput.ReadToEnd()
$err = $proc.StandardError.ReadToEnd()
$proc.WaitForExit()
($out + "`n=== STDERR ===`n" + $err) | Out-File -FilePath $logPath -Encoding utf8

"Dump copy:  $dumpCopy"
"Full log:   $logPath"
""
$markers = 'Access violation|EXCEPTION_CODE|FAULTING_IP|MODULE_NAME|IMAGE_NAME|SYMBOL_NAME|FAILURE_BUCKET_ID|STACK_TEXT'
$lines = $out -split "`r?`n"
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match $markers) {
        $lines[$i]
        if ($lines[$i] -match 'STACK_TEXT') {
            # kb frames follow directly after this marker; print the next block.
            $j = $i + 1
            while ($j -lt $lines.Count -and $lines[$j].Trim() -ne '' -and $lines[$j] -notmatch '^[A-Z_]+:') {
                $lines[$j]
                $j++
            }
        }
    }
}
