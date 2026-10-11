# Walks the open item list from top to bottom and saves one still screenshot
# per scroll step, for labeling fixtures. The game window must be visible on
# screen (it is copied from the screen) with the item list open.
# Scrolling goes through windows-link's mouse board (POST /buttons/{id}/wheel);
# while the user holds automation there, this waits.
#   powershell -File tools/capture/walk-list.ps1 -OutDir <dir>
param(
  [Parameter(Mandatory)][string]$OutDir,
  [int]$NotchesPerStep = 3,
  [int]$MaxSteps = 200,
  [string]$Link = 'http://127.0.0.1:4730',
  [string]$Button = 'blue-archive'
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class AoiWindow {
  public struct RECT { public int L, T, R, B; }
  public struct POINT { public int X, Y; }
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
}
"@
[AoiWindow]::SetProcessDPIAware() | Out-Null
New-Item -ItemType Directory -Force $OutDir | Out-Null

# Turns the wheel over the item list, waiting while the user holds automation.
function Wheel([int]$notches) {
  $body = @{ x = 0.75; y = 0.55; notches = $notches } | ConvertTo-Json
  while ($true) {
    try {
      Invoke-RestMethod -Method Post -Uri "$Link/buttons/$Button/wheel" -ContentType 'application/json' -Body $body | Out-Null
      return
    } catch {
      $answer = $_.ErrorDetails.Message | ConvertFrom-Json -ErrorAction SilentlyContinue
      if ($answer.error -ne 'user_operating') { throw }
      Write-Host 'waiting: the user is operating the game'
      Start-Sleep -Seconds 10
    }
  }
}

function Grab {
  $game = Get-Process -Name BlueArchive | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero } | Select-Object -First 1
  $rect = New-Object AoiWindow+RECT
  [AoiWindow]::GetClientRect($game.MainWindowHandle, [ref]$rect) | Out-Null
  $origin = New-Object AoiWindow+POINT
  [AoiWindow]::ClientToScreen($game.MainWindowHandle, [ref]$origin) | Out-Null
  $bitmap = New-Object System.Drawing.Bitmap $rect.R, $rect.B
  $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
  $graphics.CopyFromScreen($origin.X, $origin.Y, 0, 0, $bitmap.Size)
  $graphics.Dispose()
  $bitmap
}

# A small grey thumbnail of the list side, to tell whether the list moved.
function Thumbnail($bitmap) {
  $small = New-Object System.Drawing.Bitmap 48, 40
  $graphics = [System.Drawing.Graphics]::FromImage($small)
  $source = New-Object System.Drawing.Rectangle ([int]($bitmap.Width * 0.5)), ([int]($bitmap.Height * 0.15)), ([int]($bitmap.Width * 0.5)), ([int]($bitmap.Height * 0.65))
  $graphics.DrawImage($bitmap, (New-Object System.Drawing.Rectangle 0, 0, 48, 40), $source, [System.Drawing.GraphicsUnit]::Pixel)
  $graphics.Dispose()
  $values = foreach ($y in 0..39) { foreach ($x in 0..47) { $c = $small.GetPixel($x, $y); $c.R + $c.G + $c.B } }
  $small.Dispose()
  , $values
}

function Distance($a, $b) {
  $sum = 0
  for ($i = 0; $i -lt $a.Count; $i++) { $sum += [math]::Abs($a[$i] - $b[$i]) }
  $sum / $a.Count
}

# Waits until the list stops moving and returns that frame.
function Still {
  $previous = $null
  while ($true) {
    $bitmap = Grab
    $thumb = Thumbnail $bitmap
    if ($previous -and (Distance $previous $thumb) -lt 3) { return @($bitmap, $thumb) }
    $bitmap.Dispose()
    $previous = $thumb
    Start-Sleep -Milliseconds 300
  }
}

foreach ($i in 1..10) { Wheel 20 }   # back to the top
$last = $null
for ($step = 1; $step -le $MaxSteps; $step++) {
  $frame = Still
  if ($last -and (Distance $last $frame[1]) -lt 3) { $frame[0].Dispose(); break }   # the end
  $path = Join-Path $OutDir ('step_{0:D3}.png' -f $step)
  $frame[0].Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $frame[0].Dispose()
  $last = $frame[1]
  "saved $path"
  Wheel (-$NotchesPerStep)
}
