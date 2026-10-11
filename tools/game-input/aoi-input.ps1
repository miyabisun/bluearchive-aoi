# Elevated half of the game input helper, started by the "aoi-game-input"
# scheduled task. Blue Archive runs as administrator, so UIPI drops input sent
# from a normal process. This script reads one request written by send.ps1,
# acts only on the Blue Archive window, and writes the result next to it.
$ErrorActionPreference = 'Stop'
$dir = Join-Path $env:LOCALAPPDATA 'aoi-dev'
$requestPath = Join-Path $dir 'input.json'
$resultPath = Join-Path $dir 'input-result.json'
# Present while the user plays the game; the pause-button window creates it.
$pausePath = Join-Path $dir 'user-operating'
$sent = 0

function Finish([bool]$ok, [string]$message, [bool]$paused = $false) {
  @{ ok = $ok; paused = $paused; sent = $sent; message = $message; at = (Get-Date).ToString('o') } |
    ConvertTo-Json | Set-Content -Encoding utf8 $resultPath
  exit
}

Add-Type @"
using System; using System.Runtime.InteropServices;
public static class AoiInput {
  public struct RECT { public int L, T, R, B; }
  public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll", SetLastError = true)] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
  // Windows only lets the foreground process change focus; a tapped Alt lifts that lock.
  public static void Focus(IntPtr h) {
    keybd_event(0x12, 0, 0, UIntPtr.Zero);
    keybd_event(0x12, 0, 2, UIntPtr.Zero);
    SetForegroundWindow(h);
  }
}
"@

try {
  if (-not (Test-Path $requestPath)) { Finish $false 'no request' }
  $request = Get-Content -Raw $requestPath | ConvertFrom-Json
  Remove-Item $requestPath

  # The request file is user-writable, so every field is checked before use.
  if ($request.action -ne 'wheel') { Finish $false "unknown action: $($request.action)" }
  if ($request.mode -notin @('post', 'input')) { Finish $false "unknown mode: $($request.mode)" }
  $x = [double]$request.x; $y = [double]$request.y
  if ($x -lt 0 -or $x -gt 1 -or $y -lt 0 -or $y -gt 1) { Finish $false 'x and y must be fractions of the client area' }
  $notches = [int]$request.notches; $count = [int]$request.count; $interval = [int]$request.interval_ms
  if ($notches -eq 0 -or [math]::Abs($notches) -gt 20) { Finish $false 'notches must be 1..20 in either direction' }
  if ($count -lt 1 -or $count -gt 200) { Finish $false 'count must be 1..200' }
  if ($interval -lt 0 -or $interval -gt 2000) { Finish $false 'interval_ms must be 0..2000' }
  if (Test-Path $pausePath) { Finish $false 'the user is operating the game' $true }

  [AoiInput]::SetProcessDPIAware() | Out-Null
  $game = Get-Process -Name BlueArchive -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
  if (-not $game) { Finish $false 'Blue Archive window not found' }
  $hwnd = $game.MainWindowHandle

  $rect = New-Object AoiInput+RECT
  [AoiInput]::GetClientRect($hwnd, [ref]$rect) | Out-Null
  $point = New-Object AoiInput+POINT
  $point.X = [int]($rect.R * $x); $point.Y = [int]($rect.B * $y)
  [AoiInput]::ClientToScreen($hwnd, [ref]$point) | Out-Null
  $delta = $notches * 120

  if ($request.mode -eq 'post') {
    $wParam = [IntPtr]([int]($delta -shl 16))
    $lParam = [IntPtr]([int](($point.Y -shl 16) -bor ($point.X -band 0xFFFF)))
    for ($i = 0; $i -lt $count; $i++) {
      if (Test-Path $pausePath) { Finish $false 'the user is operating the game' $true }
      if (-not [AoiInput]::PostMessage($hwnd, 0x020A, $wParam, $lParam)) {
        Finish $false "PostMessage failed: $([Runtime.InteropServices.Marshal]::GetLastWin32Error())"
      }
      $sent++
      Start-Sleep -Milliseconds $interval
    }
  } else {
    # Real wheel input reaches whatever is under the cursor, so the game is
    # brought forward briefly and the user's cursor and focus are put back.
    $savedCursor = New-Object AoiInput+POINT
    [AoiInput]::GetCursorPos([ref]$savedCursor) | Out-Null
    $savedWindow = [AoiInput]::GetForegroundWindow()
    [AoiInput]::Focus($hwnd)
    [AoiInput]::SetCursorPos($point.X, $point.Y) | Out-Null
    Start-Sleep -Milliseconds 50
    try {
      for ($i = 0; $i -lt $count; $i++) {
        if (Test-Path $pausePath) { break }
        [AoiInput]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero)
        $sent++
        Start-Sleep -Milliseconds $interval
      }
    } finally {
      [AoiInput]::SetCursorPos($savedCursor.X, $savedCursor.Y) | Out-Null
      if ($savedWindow -ne [IntPtr]::Zero -and $savedWindow -ne $hwnd) { [AoiInput]::Focus($savedWindow) }
    }
    if ($sent -lt $count) { Finish $false 'the user is operating the game' $true }
  }
  Finish $true "sent $sent x $notches notch(es) by $($request.mode)"
} catch {
  Finish $false $_.Exception.Message
}
