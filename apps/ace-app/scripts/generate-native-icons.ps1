#!/usr/bin/env pwsh
# Generate only the native desktop icon set from the checked-in Ace dark artwork.
# Mobile assets and UI logos are intentionally left unchanged.
$ErrorActionPreference = "Stop"

$appRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$iconDir = Join-Path $appRoot "src-tauri/icons"
$brandDir = Join-Path $appRoot "src-tauri/branding"
$sourcePng = Join-Path $brandDir "ace-icon-dark.png"
$sourceSvg = Join-Path $brandDir "ace-icon-dark.svg"

foreach ($file in @($sourcePng, $sourceSvg)) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Native app icon source missing: $file"
    }
}
$staging = Join-Path ([IO.Path]::GetTempPath()) ("ace-dark-icons-" + [Guid]::NewGuid().ToString("N"))
New-Item -Path $staging -ItemType Directory -Force | Out-Null

try {
    Push-Location $appRoot
    try {
        & bun run tauri icon $sourcePng --output $staging --ios-color '#1E2936'
        if ($LASTEXITCODE -ne 0) {
            throw "Tauri native icon generation failed with exit code $LASTEXITCODE."
        }
    } finally {
        Pop-Location
    }

    $desktopFiles = @(
        "icon.png", "icon.ico", "icon.icns",
        "32x32.png", "64x64.png", "128x128.png", "128x128@2x.png",
        "Square30x30Logo.png", "Square44x44Logo.png", "Square71x71Logo.png",
        "Square89x89Logo.png", "Square107x107Logo.png", "Square142x142Logo.png",
        "Square150x150Logo.png", "Square284x284Logo.png", "Square310x310Logo.png",
        "StoreLogo.png"
    )
    foreach ($name in $desktopFiles) {
        $generated = Join-Path $staging $name
        if (-not (Test-Path -LiteralPath $generated -PathType Leaf)) {
            throw "Tauri did not generate required desktop icon: $name"
        }
    }

    foreach ($name in $desktopFiles) {
        Copy-Item -LiteralPath (Join-Path $staging $name) -Destination (Join-Path $iconDir $name) -Force
    }
    Copy-Item -LiteralPath $sourceSvg -Destination (Join-Path $iconDir "icon.svg") -Force
    Write-Host ("Updated {0} desktop icon files + icon.svg from ace-icon-dark." -f $desktopFiles.Count)
} finally {
    if (Test-Path -LiteralPath $staging) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
