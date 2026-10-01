param([string]$Player = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Player) { $Player = Join-Path $root 'target/release/yyplayer.exe' }
if (-not (Test-Path -LiteralPath $Player)) { throw 'Build the player first' }
$fixture = Join-Path $root 'target/ui-window-surface'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms,System.Drawing @"
using System;
using System.Runtime.InteropServices;
public static class YYSurfaceTest {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out Rect r);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int w, int height, uint flags);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr c);
 [DllImport("user32.dll")] public static extern IntPtr SetThreadDpiAwarenessContext(IntPtr c);
 [DllImport("dwmapi.dll")] public static extern int DwmFlush();
 [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr h);
 [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr h, IntPtr dc);
 [DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr dst,int x,int y,int w,int h,IntPtr src,int sx,int sy,uint op);
 [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attribute, out Rect value, int size);
 public static void Place(IntPtr app) {
  var prior=SetThreadDpiAwarenessContext(new IntPtr(-4));
  try { SetWindowPos(app,IntPtr.Zero,120,120,1800,1150,0x40); SetForegroundWindow(app); }
  finally { SetThreadDpiAwarenessContext(prior); }
 }
 public static void Capture(IntPtr app, IntPtr backdrop, string path) {
  var prior=SetThreadDpiAwarenessContext(new IntPtr(-4));
  try {
   Rect r,b;
   if (DwmGetWindowAttribute(app,9,out r,16)<0 || DwmGetWindowAttribute(backdrop,9,out b,16)<0) throw new Exception("Window bounds unavailable");
   var crop=new System.Drawing.Rectangle(r.Left-40,r.Top-40,480,360);
   var bg=new System.Drawing.Rectangle(b.Left,b.Top,b.Right-b.Left,b.Bottom-b.Top);
   if (!bg.Contains(crop)) throw new Exception("Capture exceeds controlled backdrop/screen");
   DwmFlush();
   using (var bitmap=new System.Drawing.Bitmap(crop.Width,crop.Height))
   using (var g=System.Drawing.Graphics.FromImage(bitmap)) {
    var screen=GetDC(IntPtr.Zero); var dest=g.GetHdc();
    try { if(!BitBlt(dest,0,0,crop.Width,crop.Height,screen,crop.X,crop.Y,0x00CC0020)) throw new Exception("Screen capture failed"); }
    finally {g.ReleaseHdc(dest); ReleaseDC(IntPtr.Zero,screen);}
    Console.WriteLine("Physical window: {0},{1} {2}x{3}",r.Left,r.Top,r.Right-r.Left,r.Bottom-r.Top);
    bitmap.Save(path,System.Drawing.Imaging.ImageFormat.Png);
   }
  } finally { SetThreadDpiAwarenessContext(prior); }
 }

}
"@
[YYSurfaceTest]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$previousDpi = [YYSurfaceTest]::SetThreadDpiAwarenessContext([IntPtr](-4))
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
# Controlled, process-owned plain backdrop. Capture includes no user desktop/media; corner crop only, not a full-window DPI qualification.
$backdrop = New-Object System.Windows.Forms.Form
$backdrop.FormBorderStyle = 'None'
$backdrop.StartPosition = 'Manual'
$backdrop.Location = New-Object System.Drawing.Point(20,20)
$backdrop.Size = New-Object System.Drawing.Size(($screen.Width-40),($screen.Height-40))
$backdrop.BackColor = [System.Drawing.Color]::FromArgb(113,139,165)
$names = @('YYPLAYER_CONFIG','YYPLAYER_SMOKE_SECONDS','YYPLAYER_DIAGNOSTICS')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
$process = $null
try {
 [IO.File]::WriteAllText((Join-Path $fixture 'settings.json'),'{"appearance":{"scheme":"Dark","design":"Fashion"}}')
 $env:YYPLAYER_CONFIG = Join-Path $fixture 'settings.json'
 $env:YYPLAYER_SMOKE_SECONDS = '14'
 $env:YYPLAYER_DIAGNOSTICS = Join-Path $fixture 'surface.json'
 $backdrop.Show()
 [System.Windows.Forms.Application]::DoEvents()
 $process = Start-Process -FilePath $Player -WorkingDirectory $root -WindowStyle Hidden -PassThru
 $deadline = [DateTime]::UtcNow.AddSeconds(8)
 do {
  Start-Sleep -Milliseconds 150
  [System.Windows.Forms.Application]::DoEvents()
  $process.Refresh()
 } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
 if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'Player window did not appear' }
 [YYSurfaceTest]::Place($process.MainWindowHandle)
 $deadline = [DateTime]::UtcNow.AddSeconds(3)
 while ([DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100; [System.Windows.Forms.Application]::DoEvents() }
 [YYSurfaceTest]::Capture($process.MainWindowHandle,$backdrop.Handle,(Join-Path $fixture 'native-corner.png'))
 if (-not $process.WaitForExit(20000)) { throw 'Window test did not exit normally' }
 if ($process.ExitCode -ne 0) { throw 'Player window test failed' }
 $report = Get-Content -LiteralPath (Join-Path $fixture 'surface.json') -Raw -Encoding UTF8 | ConvertFrom-Json
 if ($report.window_surface.result -ne 0 -or $report.window_surface.preference -ne 2 -or $report.decorated) { throw 'Native surface preference not confirmed' }
 Write-Output 'PASS: native Windows window surface; inspect native-corner.png for actual corners/shadow'
} finally {
 if ($process -and -not $process.HasExited) { $process.CloseMainWindow() | Out-Null; $process.WaitForExit(5000) | Out-Null }
 $backdrop.Close(); $backdrop.Dispose()
 [YYSurfaceTest]::SetThreadDpiAwarenessContext($previousDpi) | Out-Null
 foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name,$previous[$name],'Process') }
}
