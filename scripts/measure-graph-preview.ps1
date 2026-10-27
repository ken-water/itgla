param([int]$AppId,[string]$Label)
$ErrorActionPreference='Stop'
Add-Type 'using System; using System.Runtime.InteropServices; public class GraphProbe { [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l); [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); }'
$p=Get-Process -Id $AppId
[GraphProbe]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
[GraphProbe]::SetCursorPos(650,500) | Out-Null
Start-Sleep -Milliseconds 300
$cpu=$p.TotalProcessorTime.TotalMilliseconds
$timer=[Diagnostics.Stopwatch]::StartNew()
$latencies=@()
for($i=0;$i -lt 160;$i++) {
  $t=[Diagnostics.Stopwatch]::StartNew()
  $delta=if(($i % 40) -lt 20){-120}else{120}
  [GraphProbe]::SendMessage($p.MainWindowHandle,0x20A,[IntPtr]($delta*65536),[IntPtr](650+500*65536)) | Out-Null
  $latencies+=$t.Elapsed.TotalMilliseconds
  Start-Sleep -Milliseconds 12
}
Start-Sleep -Milliseconds 300
$p.Refresh()
$sorted=$latencies | Sort-Object
$result=[ordered]@{label=$Label;pid=$AppId;events=160;wall_ms=[math]::Round($timer.Elapsed.TotalMilliseconds,1);cpu_ms=[math]::Round($p.TotalProcessorTime.TotalMilliseconds-$cpu,1);dispatch_p95_ms=[math]::Round($sorted[151],2);working_set_mb=[math]::Round($p.WorkingSet64/1MB,1);scope='Native demo, 1400x950 physical, software renderer, wheel round trips. Not frame latency or FPS.'}
$result | ConvertTo-Json | Tee-Object -FilePath "$env:TEMP/itgla-graph-$Label-20261026.json"
