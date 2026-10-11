#Requires -RunAsAdministrator
# One-time setup, run from an administrator PowerShell. Copies the elevated
# helper to a folder normal processes cannot modify and registers the
# "aoi-game-input" task that send.ps1 starts without a UAC prompt.
$ErrorActionPreference = 'Stop'
$dest = Join-Path $env:ProgramData 'aoi-dev'
New-Item -ItemType Directory -Force $dest | Out-Null
# Administrators and SYSTEM may write; Users may only read and run.
icacls $dest /inheritance:r /grant:r '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'icacls failed' }
Copy-Item (Join-Path $PSScriptRoot 'aoi-input.ps1') $dest -Force

$script = Join-Path $dest 'aoi-input.ps1'
$action = New-ScheduledTaskAction -Execute 'conhost.exe' `
  -Argument "--headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$script`""
$principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
  -ExecutionTimeLimit (New-TimeSpan -Minutes 10) -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName 'aoi-game-input' -Action $action -Principal $principal -Settings $settings -Force | Out-Null

# Let the user's normal, unelevated session start the task.
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$service = New-Object -ComObject Schedule.Service
$service.Connect()
$task = $service.GetFolder('\').GetTask('aoi-game-input')
$dacl = $task.GetSecurityDescriptor(4)
if ($dacl -notmatch [regex]::Escape($sid)) { $task.SetSecurityDescriptor($dacl + "(A;;GRGX;;;$sid)", 0) }
"installed $script as the aoi-game-input task"
