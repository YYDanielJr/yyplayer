$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$iconRoot = Join-Path $projectRoot 'assets/icons/yyplayer/liquid-orbit-disc'
$outputPath = Join-Path $iconRoot 'yyplayer.ico'
$sizes = @(16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
$frames = @()

foreach ($size in $sizes) {
    $pngPath = Join-Path $iconRoot "yyplayer-$size.png"
    if (-not (Test-Path -LiteralPath $pngPath -PathType Leaf)) {
        throw "Required app icon frame is missing: $pngPath"
    }

    $png = [IO.File]::ReadAllBytes($pngPath)
    if ($png.Length -lt 24 -or $png[0] -ne 0x89 -or $png[1] -ne 0x50 -or $png[2] -ne 0x4E -or $png[3] -ne 0x47) {
        throw "Invalid PNG icon frame: $pngPath"
    }

    $width = ([uint32]$png[16] -shl 24) -bor ([uint32]$png[17] -shl 16) -bor ([uint32]$png[18] -shl 8) -bor [uint32]$png[19]
    $height = ([uint32]$png[20] -shl 24) -bor ([uint32]$png[21] -shl 16) -bor ([uint32]$png[22] -shl 8) -bor [uint32]$png[23]
    if ($width -ne $size -or $height -ne $size) {
        throw "PNG frame dimensions do not match its filename: $pngPath ($width x $height)"
    }

    $frames += ,([pscustomobject]@{ Size = $size; Bytes = $png })
}

$stream = [IO.MemoryStream]::new()
$writer = [IO.BinaryWriter]::new($stream)
try {
    $writer.Write([uint16]0) # reserved
    $writer.Write([uint16]1) # image resource
    $writer.Write([uint16]$frames.Count)

    $offset = 6 + 16 * $frames.Count
    foreach ($frame in $frames) {
        if ($frame.Size -eq 256) {
            $dimension = [byte]0
        } else {
            $dimension = [byte]$frame.Size
        }
        $writer.Write($dimension)
        $writer.Write($dimension)
        $writer.Write([byte]0) # palette colors
        $writer.Write([byte]0) # reserved
        $writer.Write([uint16]1) # color planes
        $writer.Write([uint16]32) # bits per pixel
        $writer.Write([uint32]$frame.Bytes.Length)
        $writer.Write([uint32]$offset)
        $offset += $frame.Bytes.Length
    }

    foreach ($frame in $frames) { $writer.Write([byte[]]$frame.Bytes) }
    $writer.Flush()
    [IO.File]::WriteAllBytes($outputPath, $stream.ToArray())
}
finally {
    $writer.Dispose()
    $stream.Dispose()
}

Write-Output "Created multi-size Windows icon: $outputPath"
