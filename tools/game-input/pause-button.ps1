# Small desktop window with the "ユーザー操作中" toggle. While it is on, the
# game input helper sends nothing, so the user can play without the automated
# development loop scrolling the game. send.ps1 starts this window when needed.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$created = $false
$mutex = New-Object System.Threading.Mutex($true, 'Local\aoi-dev-pause-button', [ref]$created)
if (-not $created) { exit }

$dir = Join-Path $env:LOCALAPPDATA 'aoi-dev'
New-Item -ItemType Directory -Force $dir | Out-Null
$pausePath = Join-Path $dir 'user-operating'

Add-Type -Namespace AoiDev -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[AoiDev.Dpi]::SetProcessDPIAware() | Out-Null
[System.Windows.Forms.Application]::EnableVisualStyles()
$form = New-Object System.Windows.Forms.Form
$form.AutoScaleDimensions = New-Object System.Drawing.SizeF(96, 96)
$form.AutoScaleMode = 'Dpi'
$form.Text = 'aoi 自動操作'
$form.ClientSize = New-Object System.Drawing.Size(340, 150)
$form.FormBorderStyle = 'FixedSingle'
$form.MaximizeBox = $false
$form.StartPosition = 'CenterScreen'
$form.Font = New-Object System.Drawing.Font('Yu Gothic UI', 11)

$status = New-Object System.Windows.Forms.Label
$status.SetBounds(16, 12, 308, 44)
$status.TextAlign = 'MiddleCenter'
$form.Controls.Add($status)

$button = New-Object System.Windows.Forms.Button
$button.SetBounds(16, 66, 308, 68)
$button.Font = New-Object System.Drawing.Font('Yu Gothic UI', 14, [System.Drawing.FontStyle]::Bold)
$form.Controls.Add($button)

function Show-State {
  if (Test-Path $pausePath) {
    $status.Text = "ユーザー操作中`n自動操作を止めています"
    $status.BackColor = [System.Drawing.Color]::FromArgb(255, 224, 178)
    $button.Text = '自動操作を再開する'
  } else {
    $status.Text = "自動操作: 許可中`nゲームを触る前に押してください"
    $status.BackColor = [System.Drawing.Color]::FromArgb(200, 230, 201)
    $button.Text = 'ユーザー操作中にする'
  }
}

$button.Add_Click({
  if (Test-Path $pausePath) { Remove-Item $pausePath } else { New-Item -ItemType File -Force $pausePath | Out-Null }
  Show-State
})
# Another copy of the flag may change it, so the window keeps itself current.
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 1000
$timer.Add_Tick({ Show-State })
$timer.Start()

Show-State
[System.Windows.Forms.Application]::Run($form)
$mutex.ReleaseMutex()
