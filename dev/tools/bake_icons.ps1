# dev/tools/bake_icons.ps1
# Native Windows In-House Multi-Resolution Icon & Asset Baker
# Generates .ico (256, 128, 64, 48, 32, 16) and master 512x512 .png files from pure vector code.

Add-Type -AssemblyName System.Drawing

function New-SquirclePath {
    param([float]$x, [float]$y, [float]$w, [float]$h, [float]$r)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = $r * 2
    $path.AddArc($x, $y, $d, $d, 180, 90)
    $path.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
    $path.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
    $path.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
    $path.CloseFigure()
    return $path
}

function Save-MultiResIcon {
    param(
        [System.Drawing.Bitmap]$MasterBitmap,
        [string]$OutputIcoPath,
        [string]$OutputPngPath
    )

    # Save high-res master PNG
    $MasterBitmap.Save($OutputPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host "  [+] Saved Master PNG: $OutputPngPath"

    $sizes = @(256, 128, 64, 48, 32, 16)
    $pngBytesList = @()

    foreach ($s in $sizes) {
        $resized = New-Object System.Drawing.Bitmap $s, $s
        $g = [System.Drawing.Graphics]::FromImage($resized)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.DrawImage($MasterBitmap, 0, 0, $s, $s)
        $g.Dispose()

        $ms = New-Object System.IO.MemoryStream
        $resized.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $resized.Dispose()
        $pngBytesList += ,$ms.ToArray()
        $ms.Dispose()
    }

    $fs = [System.IO.File]::Create($OutputIcoPath)
    $bw = New-Object System.IO.BinaryWriter $fs

    # Header
    $bw.Write([uint16]0) # Reserved
    $bw.Write([uint16]1) # Type (1 = icon)
    $bw.Write([uint16]$sizes.Count) # Count

    # Calculate offsets
    $offset = 6 + (16 * $sizes.Count)

    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $s = $sizes[$i]
        $bytes = $pngBytesList[$i]
        $w = if ($s -eq 256) { 0 } else { [byte]$s }
        $h = if ($s -eq 256) { 0 } else { [byte]$s }

        $bw.Write([byte]$w)
        $bw.Write([byte]$h)
        $bw.Write([byte]0)   # Colors
        $bw.Write([byte]0)   # Reserved
        $bw.Write([uint16]1) # Planes
        $bw.Write([uint16]32)# Bits
        $bw.Write([uint32]$bytes.Length)
        $bw.Write([uint32]$offset)

        $offset += $bytes.Length
    }

    # Write PNG payloads
    for ($i = 0; $i -lt $sizes.Count; $i++) {
        $bw.Write($pngBytesList[$i])
    }

    $bw.Flush()
    $bw.Dispose()
    $fs.Dispose()
    Write-Host "  [+] Saved Multi-Res ICO: $OutputIcoPath"
}

# ==============================================================================
# 1. BAKE AARONEOUS CORE ICON
# ==============================================================================
Write-Host "Baking Aaroneous Core Suite Icon..."
$bmpCore = New-Object System.Drawing.Bitmap 512, 512
$g = [System.Drawing.Graphics]::FromImage($bmpCore)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Background Squircle
$bgPath = New-SquirclePath 20 20 472 472 108
$bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(0, 0),
    [System.Drawing.PointF]::new(512, 512),
    [System.Drawing.Color]::FromArgb(255, 26, 30, 38),
    [System.Drawing.Color]::FromArgb(255, 10, 11, 14)
)
$g.FillPath($bgBrush, $bgPath)
$borderPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 40, 46, 59)), 4
$g.DrawPath($borderPen, $bgPath)

# Outer Hexagonal Lattice Points (Center 256, 256, Radius 185)
$hexOuter = @(
    [System.Drawing.PointF]::new(256, 71),
    [System.Drawing.PointF]::new(416, 163),
    [System.Drawing.PointF]::new(416, 348),
    [System.Drawing.PointF]::new(256, 441),
    [System.Drawing.PointF]::new(96, 348),
    [System.Drawing.PointF]::new(96, 163)
)
$latticePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 34, 40, 52)), 8
$g.DrawPolygon($latticePen, $hexOuter)

# Glowing Cyan Interconnect Lines & Vertices
$cyanPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 229, 255)), 4
$cyanPen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dash
$g.DrawPolygon($cyanPen, $hexOuter)

# Bus Channels to Core
$busPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 166, 255)), 4
$busPen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Solid
$g.DrawLine($busPen, 256, 71, 256, 175)
$g.DrawLine($busPen, 256, 441, 256, 337)
$g.DrawLine($busPen, 96, 163, 186, 215)
$g.DrawLine($busPen, 416, 163, 326, 215)
$g.DrawLine($busPen, 96, 348, 186, 296)
$g.DrawLine($busPen, 416, 348, 326, 296)

# Outer Node Nodes
$cyanBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 0, 229, 255))
$purpleBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 112, 89, 255))
foreach ($pt in $hexOuter) {
    $g.FillEllipse($cyanBrush, $pt.X - 9, $pt.Y - 9, 18, 18)
}

# Thermodynamic Core Hexagon (Center 256, 256, Radius 120)
$hexInner = @(
    [System.Drawing.PointF]::new(256, 136),
    [System.Drawing.PointF]::new(360, 196),
    [System.Drawing.PointF]::new(360, 316),
    [System.Drawing.PointF]::new(256, 376),
    [System.Drawing.PointF]::new(152, 316),
    [System.Drawing.PointF]::new(152, 196)
)
$coreDarkBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 20, 23, 30))
$g.FillPolygon($coreDarkBrush, $hexInner)

$rustPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 242, 117, 94)), 8
$g.DrawPolygon($rustPen, $hexInner)

# Inner Rings
$matrixPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 229, 255)), 3
$matrixPen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dot
$g.DrawEllipse($matrixPen, 256 - 76, 256 - 76, 152, 152)

# Central Quantum Singularity
$singularityBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(210, 210),
    [System.Drawing.PointF]::new(302, 302),
    [System.Drawing.Color]::FromArgb(255, 255, 179, 112),
    [System.Drawing.Color]::FromArgb(255, 158, 42, 43)
)
$g.FillEllipse($singularityBrush, 256 - 46, 256 - 46, 92, 92)
$whiteBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(240, 255, 255, 255))
$g.FillEllipse($whiteBrush, 256 - 20, 256 - 20, 40, 40)
$g.FillEllipse([System.Drawing.Brushes]::White, 256 - 8, 256 - 8, 16, 16)

$g.Dispose()

Save-MultiResIcon -MasterBitmap $bmpCore -OutputIcoPath "D:\Aaroneous\assets\icons\aaroneous_core.ico" -OutputPngPath "D:\Aaroneous\assets\icons\aaroneous_core.png"
$bmpCore.Dispose()

# ==============================================================================
# 2. BAKE AARONEOUS FLIGHT CONTROLLER (AFC) ICON
# ==============================================================================
Write-Host "Baking Aaroneous Flight Controller (AFC) Icon..."
$bmpAfc = New-Object System.Drawing.Bitmap 512, 512
$g = [System.Drawing.Graphics]::FromImage($bmpAfc)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

# Background Squircle
$bgPath = New-SquirclePath 20 20 472 472 108
$bgBrushAfc = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(0, 0),
    [System.Drawing.PointF]::new(512, 512),
    [System.Drawing.Color]::FromArgb(255, 20, 23, 32),
    [System.Drawing.Color]::FromArgb(255, 7, 8, 11)
)
$g.FillPath($bgBrushAfc, $bgPath)
$borderPenAfc = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 31, 37, 51)), 4
$g.DrawPath($borderPenAfc, $bgPath)

# Compass / Telemetry Reticle Rings
$darkReticlePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 29, 35, 48)), 6
$g.DrawEllipse($darkReticlePen, 256 - 184, 256 - 184, 368, 368)

$cyanReticlePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 5
$cyanReticlePen.DashPattern = @(10.0, 3.0, 15.0, 3.0)
$g.DrawEllipse($cyanReticlePen, 256 - 184, 256 - 184, 368, 368)

# Pitch Indices (Ticks)
$tickPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 4
$tickPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$tickPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$g.DrawLine($tickPen, 64, 256, 112, 256)
$g.DrawLine($tickPen, 400, 256, 448, 256)
$g.DrawLine($tickPen, 256, 64, 256, 112)
$g.DrawLine($tickPen, 256, 400, 256, 448)

# 45-degree Guide Vector
$vectorGuidePen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 34, 42, 58)), 3
$vectorGuidePen.DashStyle = [System.Drawing.Drawing2D.DashStyle]::Dash
$g.DrawLine($vectorGuidePen, 120, 392, 392, 120)

# Thermal Flare Afterburner (Behind Chevron)
$thrustPoints = @(
    [System.Drawing.PointF]::new(270, 225),
    [System.Drawing.PointF]::new(210, 340),
    [System.Drawing.PointF]::new(235, 270)
)
$thrustBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush (
    [System.Drawing.PointF]::new(210, 340),
    [System.Drawing.PointF]::new(270, 225),
    [System.Drawing.Color]::FromArgb(255, 255, 85, 0),
    [System.Drawing.Color]::FromArgb(255, 255, 230, 128)
)
$g.FillPolygon($thrustBrush, $thrustPoints)

# Tactical Flight Chevron (Points Elevate-Right)
$chevronPoints = @(
    [System.Drawing.PointF]::new(330, 110),
    [System.Drawing.PointF]::new(390, 260),
    [System.Drawing.PointF]::new(270, 225),
    [System.Drawing.PointF]::new(210, 340),
    [System.Drawing.PointF]::new(180, 310),
    [System.Drawing.PointF]::new(245, 210),
    [System.Drawing.PointF]::new(170, 195)
)
$chevronDarkBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 16, 20, 29))
$g.FillPolygon($chevronDarkBrush, $chevronPoints)

$chevronCyanPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 0, 240, 255)), 7
$chevronCyanPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$g.DrawPolygon($chevronCyanPen, $chevronPoints)

# Center Aim Crosshairs & Guidance Pip
$centerCrosshairPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 38, 48, 66)), 2
$g.DrawEllipse($centerCrosshairPen, 256 - 44, 256 - 44, 88, 88)
$g.FillEllipse([System.Drawing.Brushes]::White, 256 - 8, 256 - 8, 16, 16)
$g.FillEllipse($cyanBrush, 256 - 3, 256 - 3, 6, 6)

$g.Dispose()

Save-MultiResIcon -MasterBitmap $bmpAfc -OutputIcoPath "D:\Aaroneous\assets\icons\afc_controller.ico" -OutputPngPath "D:\Aaroneous\assets\icons\afc_controller.png"
$bmpAfc.Dispose()

Write-Host "`nAll icons baked successfully into D:\Aaroneous\assets\icons!"
