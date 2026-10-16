#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Rejects shipped PE binaries that depend on separately installed MSVC runtime DLLs.
.DESCRIPTION
    Uses LLVM's PE import reader, including delay imports. Universal CRT API-set
    libraries are Windows inbox components on supported Windows 10/11 releases;
    they are not Visual C++ Redistributable dependencies.
#>
param(
    [Parameter(Mandatory = $true)]
    [string[]]$Paths
)

$ErrorActionPreference = "Stop"
$reader = (Get-Command "llvm-readobj.exe" -ErrorAction SilentlyContinue).Source
if (-not $reader) {
    $candidate = Join-Path $env:ProgramFiles "LLVM\bin\llvm-readobj.exe"
    if (Test-Path -LiteralPath $candidate) {
        $reader = $candidate
    }
}
if (-not $reader) {
    throw "llvm-readobj.exe is required to verify packaged PE imports; install LLVM for Windows."
}

foreach ($path in $Paths) {
    $resolved = (Resolve-Path -LiteralPath $path).Path
    $output = @(& $reader --coff-imports $resolved 2>&1)
    if ($LASTEXITCODE -ne 0) {
        throw "Could not inspect PE imports of '$resolved': $($output -join ' ')"
    }

    $imports = @($output | ForEach-Object {
        if ($_ -match '^\s*Name:\s*([^\s]+\.dll)\s*$') {
            $Matches[1]
        }
    } | Sort-Object -Unique)
    if ($imports.Count -eq 0) {
        throw "No PE import DLLs found in '$resolved'; cannot verify CRT dependencies."
    }

    # Check actual import tables, not byte substrings (which can be innocuous).
    $redistributableImports = @($imports | Where-Object {
        $_ -match '^(?:vcruntime[a-z0-9_]*|msvcp[a-z0-9_]*|msvcr[0-9]+|concrt[a-z0-9_]*|vcomp[a-z0-9_]*|mfc[a-z0-9_]*|atl[0-9_]+)\.dll$'
    })
    if ($redistributableImports.Count -gt 0) {
        throw "'$resolved' requires Visual C++ Redistributable DLL(s): $($redistributableImports -join ', ')"
    }

    $inboxUcrt = @($imports | Where-Object {
        $_ -match '^(?:api-ms-win-crt-[a-z0-9-]+|ucrtbase)\.dll$'
    })
    Write-Host ("  CRT check OK: {0} ({1} DLL imports; {2} Windows-inbox UCRT imports)" -f
        (Split-Path -Leaf $resolved), $imports.Count, $inboxUcrt.Count)
}
