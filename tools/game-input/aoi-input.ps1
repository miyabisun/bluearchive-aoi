# Elevated half of the game input helper, started by the "aoi-game-input"
# scheduled task. Blue Archive runs as administrator, so UIPI drops input sent
# from a normal process. This script reads one request written by send.ps1,
# acts only on the Blue Archive window, and writes the result next to it.
#
# A scheduled task may start without the user's environment variables, so
# folders come from the Windows API, and failures go to input-error.log.
$ErrorActionPreference = 'Stop'
$local = [Environment]::GetFolderPath('LocalApplicationData')
$dir = Join-Path $local 'aoi-dev'
$requestPath = Join-Path $dir 'input.json'
$resultPath = Join-Path $dir 'input-result.json'
$logPath = Join-Path $dir 'input-error.log'
# Present while the user plays the game; the pause-button window creates it.
$pausePath = Join-Path $dir 'user-operating'
$sent = 0
if (-not $env:TEMP) { $env:TEMP = Join-Path $local 'Temp'; $env:TMP = $env:TEMP }

function Finish([bool]$ok, [string]$message, [bool]$paused = $false) {
  @{ ok = $ok; paused = $paused; sent = $sent; message = $message; at = (Get-Date).ToString('o') } |
    ConvertTo-Json | Set-Content -Encoding utf8 $resultPath
  exit
}

try {
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
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr pid);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, int data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  // Windows lets only the foreground thread, or the sender of the last input,
  // hand focus over. First borrow the foreground thread's input state; failing
  // that, send F24, a key no application uses, and try again.
  public static bool Focus(IntPtr h) {
    IntPtr current = GetForegroundWindow();
    if (current == h) return true;
    uint mine = GetCurrentThreadId();
    uint theirs = GetWindowThreadProcessId(current, IntPtr.Zero);
    AttachThreadInput(mine, theirs, true);
    SetForegroundWindow(h);
    BringWindowToTop(h);
    AttachThreadInput(mine, theirs, false);
    if (GetForegroundWindow() == h) return true;
    keybd_event(0x87, 0, 0, UIntPtr.Zero);
    keybd_event(0x87, 0, 2, UIntPtr.Zero);
    SetForegroundWindow(h);
    return GetForegroundWindow() == h;
  }
}
"@

  if (-not (Test-Path $requestPath)) { Finish $false 'no request' }
  $request = Get-Content -Raw $requestPath | ConvertFrom-Json
  Remove-Item $requestPath
  # install.ps1 checks that the task runs at all.
  if ($request.action -eq 'ping') { Finish $true 'pong' }

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
  if ([AoiInput]::IsIconic($hwnd)) { Finish $false 'Blue Archive is minimized' }

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
    Finish $true "sent $sent x $notches notch(es) by post"
  }

  # The game reads the wheel only while it is in front and the cursor is over
  # the list, so both are checked before anything is sent. The user's cursor
  # and window are put back afterwards.
  $savedCursor = New-Object AoiInput+POINT
  [AoiInput]::GetCursorPos([ref]$savedCursor) | Out-Null
  $savedWindow = [AoiInput]::GetForegroundWindow()
  $problem = $null
  try {
    if (-not [AoiInput]::Focus($hwnd)) {
      $problem = 'could not bring Blue Archive to the front'
    } else {
      [AoiInput]::SetCursorPos($point.X, $point.Y) | Out-Null
      Start-Sleep -Milliseconds 50
      $under = [AoiInput]::GetAncestor([AoiInput]::WindowFromPoint($point), 2)
      if ($under -ne $hwnd) {
        $problem = 'another window covers the list'
      } else {
        for ($i = 0; $i -lt $count; $i++) {
          if (Test-Path $pausePath) { break }
          [AoiInput]::mouse_event(0x0800, 0, 0, $delta, [UIntPtr]::Zero)
          $sent++
          Start-Sleep -Milliseconds $interval
        }
      }
    }
  } finally {
    [AoiInput]::SetCursorPos($savedCursor.X, $savedCursor.Y) | Out-Null
    if ($savedWindow -ne [IntPtr]::Zero -and $savedWindow -ne $hwnd) { [AoiInput]::Focus($savedWindow) | Out-Null }
  }
  if ($problem) { Finish $false $problem }
  if ($sent -lt $count) { Finish $false 'the user is operating the game' $true }
  Finish $true "sent $sent x $notches notch(es) by input"
} catch {
  "$(Get-Date -Format o) $($_.Exception.Message)" | Add-Content -Encoding utf8 $logPath
  Finish $false $_.Exception.Message
}
