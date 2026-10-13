# Writes `boxes.json`: the choice boxes of items or of equipment (one box gives
# one of its choices), which the training plan opens for what it is short of.
# Called by update.ps1.
#   powershell -File tools/schaledb/boxes.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..\..\src-tauri\data\schaledb'
Add-Type -AssemblyName System.Web.Extensions

function Get-Json($url) {
  $bytes = (Invoke-WebRequest -UseBasicParsing $url).RawContentStream.ToArray()
  $parser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
  $parser.MaxJsonLength = [int]::MaxValue
  $parser.DeserializeObject([Text.Encoding]::UTF8.GetString($bytes))
}

$items = (Get-Json 'https://schaledb.com/data/jp/items.json').Values |
  Where-Object { $_['IsReleased'][0] -and $_['ConsumeType'] -eq 'Choice' } |
  Sort-Object { [int64]$_['Id'] }
$lines = foreach ($box in $items) {
  $choices = @($box['Items'])
  $types = @($choices | ForEach-Object { $_['Type'] } | Sort-Object -Unique)
  # Furniture, and boxes of more than one of a choice, are of no use to the plan.
  if ($types.Count -ne 1 -or $types[0] -notin @('Item', 'Equipment')) { continue }
  if (@($choices | Where-Object { $_['AmountMin'] -ne 1 -or $_['AmountMax'] -ne 1 }).Count) { continue }
  [ordered]@{
    id = [int64]$box['Id']
    equipment = ($types[0] -eq 'Equipment')
    choices = @($choices | ForEach-Object { [int64]$_['Id'] })
  } | ConvertTo-Json -Compress
}
[IO.File]::WriteAllText((Join-Path $root 'boxes.json'), ("[`n" + ($lines -join ",`n") + "`n]`n"), (New-Object Text.UTF8Encoding($false)))
Write-Host "boxes: $(@($lines).Count) entries"
