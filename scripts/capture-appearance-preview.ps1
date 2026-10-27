param([int]$AppId, [string]$Name = 'services', [string]$ScreenshotScript = (Join-Path $env:USERPROFILE '.codex/skills/screenshot/scripts/take_screenshot.ps1'))
$ErrorActionPreference = 'Stop'
Add-Type 'using System; using System.Runtime.InteropServices; public class AppearanceCapture { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int z,uint flags); }'
[AppearanceCapture]::SetProcessDPIAware() | Out-Null
$handle = (Get-Process -Id $AppId).MainWindowHandle
[AppearanceCapture]::SetWindowPos($handle,[IntPtr](-1),10,10,1400,950,64) | Out-Null
Start-Sleep -Milliseconds 400
& $ScreenshotScript -WindowHandle $handle -Path "$env:TEMP/itgla-$Name-20261026.png"
[AppearanceCapture]::SetWindowPos($handle,[IntPtr](-2),0,0,0,0,19) | Out-Null
