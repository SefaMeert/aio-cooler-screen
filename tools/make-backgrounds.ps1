# Renders original 480x480 backgrounds for the round cooler screen.
param([string]$OutDir)
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force $OutDir | Out-Null
$S = 480

function New-Canvas { $b = [Drawing.Bitmap]::new($S, $S); $g = [Drawing.Graphics]::FromImage($b); $g.SmoothingMode = 'AntiAlias'; $g.InterpolationMode = 'HighQualityBicubic'; $g.CompositingQuality = 'HighQuality'; return @($b, $g) }
function C([int]$a, [int]$r, [int]$gg, [int]$bb) { [Drawing.Color]::FromArgb($a, $r, $gg, $bb) }

# Soft glow: a radial gradient blob that fades to transparent.
function Glow($g, [float]$cx, [float]$cy, [float]$r, $color, [int]$alpha) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $path.AddEllipse($cx - $r, $cy - $r, 2 * $r, 2 * $r)
    $br = New-Object Drawing.Drawing2D.PathGradientBrush $path
    $br.CenterColor = [Drawing.Color]::FromArgb($alpha, $color)
    $br.SurroundColors = @([Drawing.Color]::FromArgb(0, $color))
    $g.FillPath($br, $path); $br.Dispose(); $path.Dispose()
}

function Fill-Radial($g, $inner, $outer) {
    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $path.AddEllipse(-120, -120, $S + 240, $S + 240)
    $br = New-Object Drawing.Drawing2D.PathGradientBrush $path
    $br.CenterPoint = [Drawing.PointF]::new(240, 240)
    $br.CenterColor = $inner; $br.SurroundColors = @($outer)
    $g.FillRectangle($br, 0, 0, $S, $S); $br.Dispose(); $path.Dispose()
}

$rng = New-Object Random 7

# 1) Nebula: deep space with coloured gas clouds and stars.
$b, $g = New-Canvas
Fill-Radial $g (C 255 22 18 48) (C 255 4 4 12)
foreach ($blob in @(@(150, 170, 210, (C 255 120 60 220), 120), @(330, 300, 230, (C 255 30 140 230), 110), @(300, 140, 150, (C 255 230 70 160), 80), @(170, 350, 160, (C 255 40 200 190), 70))) {
    Glow $g $blob[0] $blob[1] $blob[2] $blob[3] $blob[4]
}
for ($i = 0; $i -lt 260; $i++) {
    $x = $rng.NextDouble() * $S; $y = $rng.NextDouble() * $S; $r = [math]::Pow($rng.NextDouble(), 3) * 1.8 + 0.4
    $a = [int](90 + $rng.NextDouble() * 165)
    $br = New-Object Drawing.SolidBrush (C $a 255 255 255); $g.FillEllipse($br, $x - $r, $y - $r, 2 * $r, 2 * $r); $br.Dispose()
}
foreach ($star in @(@(110, 120), @(360, 90), @(400, 380), @(90, 330))) { Glow $g $star[0] $star[1] 10 (C 255 255 255 255) 200 }
$g.Dispose(); $b.Save("$OutDir\nebula.png"); $b.Dispose()

# 2) Aurora: dark teal sky with flowing light bands.
$b, $g = New-Canvas
Fill-Radial $g (C 255 8 34 44) (C 255 2 8 14)
$bands = @(@(0, 230, (C 255 60 240 170)), @(40, 270, (C 255 70 180 255)), @(-30, 300, (C 255 170 110 255)))
foreach ($band in $bands) {
    for ($k = 0; $k -lt 26; $k++) {
        $off = $band[0] + $k * 3.2
        $pts = @()
        for ($x = -20; $x -le 500; $x += 20) {
            $y = $band[1] + 55 * [math]::Sin(($x + $off) / 85.0) - 30 * [math]::Cos(($x - $off) / 140.0) - $k * 4
            $pts += [Drawing.PointF]::new($x, $y)
        }
        $alpha = [int](26 * (1 - $k / 26.0) + 4)
        $pen = [Drawing.Pen]::new([Drawing.Color]::FromArgb($alpha, $band[2]), [float](14 - $k * 0.4))
        $g.DrawCurve($pen, [Drawing.PointF[]]$pts, 0.5); $pen.Dispose()
    }
}
for ($i = 0; $i -lt 120; $i++) {
    $x = $rng.NextDouble() * $S; $y = $rng.NextDouble() * 200; $r = $rng.NextDouble() * 1.1 + 0.3
    $br = New-Object Drawing.SolidBrush (C ([int](80 + $rng.NextDouble() * 140)) 230 250 255); $g.FillEllipse($br, $x - $r, $y - $r, 2 * $r, 2 * $r); $br.Dispose()
}
# Mountain silhouette
$pts = @(([Drawing.PointF]::new(0, 480)), ([Drawing.PointF]::new(0, 400)))
for ($x = 0; $x -le 480; $x += 24) { $pts += [Drawing.PointF]::new($x, 400 - 40 * [math]::Abs([math]::Sin($x / 70.0)) - 20 * $rng.NextDouble()) }
$pts += [Drawing.PointF]::new(480, 480)
$br = New-Object Drawing.SolidBrush (C 255 3 10 14); $g.FillPolygon($br, [Drawing.PointF[]]$pts); $br.Dispose()
$g.Dispose(); $b.Save("$OutDir\aurora.png"); $b.Dispose()

# 3) Neon rings: dark carbon texture with glowing concentric rings.
$b, $g = New-Canvas
Fill-Radial $g (C 255 26 26 32) (C 255 6 6 9)
for ($y = 0; $y -lt $S; $y += 6) {
    for ($x = (($y / 6) % 2) * 6; $x -lt $S; $x += 12) {
        $br = New-Object Drawing.SolidBrush (C 14 255 255 255); $g.FillRectangle($br, $x, $y, 6, 3); $br.Dispose()
    }
}
foreach ($ring in @(@(228, (C 255 255 40 120), 3), @(212, (C 255 0 220 255), 2), @(150, (C 255 140 80 255), 1.5))) {
    for ($w = 12; $w -ge 1; $w -= 1) {
        $pen = [Drawing.Pen]::new([Drawing.Color]::FromArgb([int](10 + (12 - $w) * 4), $ring[1]), [float]($ring[2] + $w * 2))
        $g.DrawEllipse($pen, 240 - $ring[0], 240 - $ring[0], 2 * $ring[0], 2 * $ring[0]); $pen.Dispose()
    }
    $pen = New-Object Drawing.Pen ([Drawing.Color]::FromArgb(255, $ring[1])), $ring[2]
    $g.DrawEllipse($pen, 240 - $ring[0], 240 - $ring[0], 2 * $ring[0], 2 * $ring[0]); $pen.Dispose()
}
Glow $g 240 240 140 (C 255 120 60 255) 50
$g.Dispose(); $b.Save("$OutDir\neon-rings.png"); $b.Dispose()

# 4) Sunset: warm gradient with a sun and layered hills.
$b, $g = New-Canvas
$br = New-Object Drawing.Drawing2D.LinearGradientBrush ([Drawing.Point]::new(0, 0)), ([Drawing.Point]::new(0, 480)), (C 255 40 20 80), (C 255 255 120 80)
$blend = New-Object Drawing.Drawing2D.ColorBlend 4
$blend.Colors = @((C 255 28 16 70), (C 255 120 40 120), (C 255 240 100 90), (C 255 255 190 110))
$blend.Positions = @(0.0, 0.45, 0.75, 1.0)
$br.InterpolationColors = $blend
$g.FillRectangle($br, 0, 0, $S, $S); $br.Dispose()
Glow $g 240 300 170 (C 255 255 210 140) 160
$br = New-Object Drawing.SolidBrush (C 255 255 236 190); $g.FillEllipse($br, 180, 240, 120, 120); $br.Dispose()
$layers = @(@(330, (C 255 120 50 110), 30), @(370, (C 255 70 30 80), 24), @(410, (C 255 36 16 50), 18))
foreach ($l in $layers) {
    $pts = @(([Drawing.PointF]::new(0, 480)))
    for ($x = 0; $x -le 480; $x += 20) { $pts += [Drawing.PointF]::new($x, $l[0] + $l[2] * [math]::Sin(($x + $l[0]) / 60.0)) }
    $pts += [Drawing.PointF]::new(480, 480)
    $br = New-Object Drawing.SolidBrush $l[1]; $g.FillPolygon($br, [Drawing.PointF[]]$pts); $br.Dispose()
}
$g.Dispose(); $b.Save("$OutDir\sunset.png"); $b.Dispose()

# Preview sheet with round masks, as the cooler shows them.
$sheet = [Drawing.Bitmap]::new(1920, 480); $g = [Drawing.Graphics]::FromImage($sheet); $g.SmoothingMode = 'AntiAlias'
$g.Clear((C 255 24 24 27)); $i = 0
foreach ($n in 'nebula', 'aurora', 'neon-rings', 'sunset') {
    $img = [Drawing.Image]::FromFile("$OutDir\$n.png")
    $path = New-Object Drawing.Drawing2D.GraphicsPath; $path.AddEllipse($i * 480 + 10, 10, 460, 460); $g.SetClip($path)
    $g.DrawImage($img, $i * 480 + 10, 10, 460, 460); $g.ResetClip(); $img.Dispose(); $i++
}
$g.Dispose(); $sheet.Save("$OutDir\_preview.png"); $sheet.Dispose()
"done"
