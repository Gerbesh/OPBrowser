# Offline, deterministic color fixtures using only the Windows infrastructure codec.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$fixtureDirectory = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../examples/images'))
New-Item -ItemType Directory -Path $fixtureDirectory -Force | Out-Null
$bitmap = [System.Drawing.Bitmap]::new(2, 2)
try {
    $bitmap.SetPixel(0, 0, [System.Drawing.Color]::FromArgb(255, 255, 0, 0))
    $bitmap.SetPixel(1, 0, [System.Drawing.Color]::FromArgb(255, 0, 255, 0))
    $bitmap.SetPixel(0, 1, [System.Drawing.Color]::FromArgb(128, 0, 0, 255))
    $bitmap.SetPixel(1, 1, [System.Drawing.Color]::FromArgb(0, 0, 0, 0))
    $bitmap.Save((Join-Path $fixtureDirectory 'colors.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Save((Join-Path $fixtureDirectory 'colors.jpg'), [System.Drawing.Imaging.ImageFormat]::Jpeg)
    $bitmap.Save((Join-Path $fixtureDirectory 'colors.gif'), [System.Drawing.Imaging.ImageFormat]::Gif)
    $bitmap.Save((Join-Path $fixtureDirectory 'colors.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
} finally { $bitmap.Dispose() }
$oversized = [System.Drawing.Bitmap]::new(8192, 1)
try { $oversized.Save((Join-Path $fixtureDirectory 'oversized.png'), [System.Drawing.Imaging.ImageFormat]::Png) }
finally { $oversized.Dispose() }
$large = [System.Drawing.Bitmap]::new(4096, 1024)
try { $large.Save((Join-Path $fixtureDirectory 'pixel-budget.png'), [System.Drawing.Imaging.ImageFormat]::Png) }
finally { $large.Dispose() }
