# Scrolls the Blue Archive window through the elevated helper without a UAC
# prompt. Needs install.ps1 to have been run once as administrator.
# While the user has "ユーザー操作中" on, this waits and then sends the rest.
#   powershell -File tools/game-input/send.ps1 -Notches -1 -Count 3
param(
  [ValidateSet('post', 'input')][string]$Mode = 'input',
  [double]$X = 0.75,
  [double]$Y = 0.55,
  [int]$Notches = -1,
  [int]$Count = 1,
  [int]$IntervalMs = 150,
  [int]$TimeoutSec = 30
)
$ErrorActionPreference = 'Stop'
$dir = Join-Path $env:LOCALAPPDATA 'aoi-dev'
New-Item -ItemType Directory -Force $dir | Out-Null
$requestPath = Join-Path $dir 'input.json'
$resultPath = Join-Path $dir 'input-result.json'
$pausePath = Join-Path $dir 'user-operating'

# The user needs the toggle whenever automation may touch the game.
$exists = $null
if (-not [System.Threading.Mutex]::TryOpenExisting('Local\aoi-dev-pause-button', [ref]$exists)) {
  # A hidden start would hide the form too, so only the console is suppressed.
  Start-Process conhost.exe -ArgumentList '--headless', 'powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass',
    '-File', "`"$PSScriptRoot\pause-button.ps1`""
}

$remaining = $Count
while ($remaining -gt 0) {
  if (Test-Path $pausePath) {
    Write-Host 'waiting: the user is operating the game'
    while (Test-Path $pausePath) { Start-Sleep -Seconds 2 }
  }
  Remove-Item $resultPath -ErrorAction SilentlyContinue
  @{ action = 'wheel'; mode = $Mode; x = $X; y = $Y; notches = $Notches; count = $remaining; interval_ms = $IntervalMs } |
    ConvertTo-Json | Set-Content -Encoding utf8 $requestPath
  schtasks.exe /run /tn aoi-game-input | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'the aoi-game-input task is missing: run tools/game-input/install.ps1 as administrator' }

  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while (-not (Test-Path $resultPath)) {
    if ((Get-Date) -gt $deadline) { throw 'the elevated helper did not answer' }
    Start-Sleep -Milliseconds 100
  }
  $result = Get-Content -Raw $resultPath | ConvertFrom-Json
  $remaining -= [int]$result.sent
  if (-not $result.ok -and -not $result.paused) { throw $result.message }
}
"sent $Count x $Notches notch(es) by $Mode"
