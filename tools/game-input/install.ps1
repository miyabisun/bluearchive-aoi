#Requires -RunAsAdministrator
# One-time setup, run from an administrator PowerShell. Copies the elevated
# helper to a folder normal processes cannot modify, registers the
# "aoi-game-input" task that send.ps1 starts without a UAC prompt, and checks
# that the task answers.
$ErrorActionPreference = 'Stop'
$dest = Join-Path $env:ProgramData 'aoi-dev'
New-Item -ItemType Directory -Force $dest | Out-Null
# Administrators and SYSTEM may write; Users may only read and run.
icacls $dest /inheritance:r /grant:r '*S-1-5-32-544:(OI)(CI)F' '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-545:(OI)(CI)RX' | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'icacls failed' }
Copy-Item (Join-Path $PSScriptRoot 'aoi-input.ps1') $dest -Force
$script = Join-Path $dest 'aoi-input.ps1'

$exchange = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'aoi-dev'
New-Item -ItemType Directory -Force $exchange | Out-Null
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value

function Register($action) {
  $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Highest
  $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
    -ExecutionTimeLimit (New-TimeSpan -Minutes 10) -MultipleInstances IgnoreNew
  Register-ScheduledTask -TaskName 'aoi-game-input' -Action $action -Principal $principal -Settings $settings -Force | Out-Null
  # Let the user's normal, unelevated session start the task.
  $service = New-Object -ComObject Schedule.Service
  $service.Connect()
  $task = $service.GetFolder('\').GetTask('aoi-game-input')
  $dacl = $task.GetSecurityDescriptor(4)
  if ($dacl -notmatch [regex]::Escape($sid)) { $task.SetSecurityDescriptor($dacl + "(A;;GRGX;;;$sid)", 0) }
}

function Answers {
  $result = Join-Path $exchange 'input-result.json'
  Remove-Item $result -ErrorAction SilentlyContinue
  '{"action":"ping"}' | Set-Content -Encoding utf8 (Join-Path $exchange 'input.json')
  Start-ScheduledTask -TaskName 'aoi-game-input'
  foreach ($i in 1..40) {
    if (Test-Path $result) { return (Get-Content -Raw $result | ConvertFrom-Json).ok }
    Start-Sleep -Milliseconds 250
  }
  $false
}

# A headless console shows no window; a hidden PowerShell flashes briefly
# but is the fallback if the headless start does not run the script.
$launchers = @(
  @{ name = 'headless console'; action = New-ScheduledTaskAction -Execute 'conhost.exe' -Argument "--headless powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$script`"" },
  @{ name = 'hidden PowerShell'; action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument "-NoProfile -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$script`"" }
)
foreach ($launcher in $launchers) {
  Register $launcher.action
  if (Answers) {
    "OK: the aoi-game-input task answers ($($launcher.name))"
    return
  }
  "the task did not answer with the $($launcher.name)"
}
$log = Join-Path $exchange 'input-error.log'
if (Test-Path $log) { Get-Content $log -Tail 5 }
throw 'the aoi-game-input task does not answer; see the lines above'
