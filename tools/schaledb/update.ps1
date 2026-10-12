# Refreshes the SchaleDB snapshot the recognizer uses: the JP items, equipment
# and students that are released in Japan, each sorted by Id (the game's
# default list order), and the item and equipment icons. Tests and the app read
# only this snapshot, never the network.
#   powershell -File tools/schaledb/update.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..\..\src-tauri\data\schaledb'

# Writes `<name>.json` (one entry per line keeps the snapshot diffable when
# SchaleDB adds entries) and fetches the icons missing from `<iconDir>` (none
# without one).
function Update-List($name, $iconDir, $iconUrl, $fields) {
  # The response has no charset, which Windows PowerShell would read as Latin-1.
  $bytes = (Invoke-WebRequest -UseBasicParsing "https://schaledb.com/data/jp/$name.json").RawContentStream.ToArray()
  # ConvertFrom-Json refuses keys that differ only in case, which students.json has.
  Add-Type -AssemblyName System.Web.Extensions
  $parser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
  $parser.MaxJsonLength = [int]::MaxValue
  $raw = $parser.DeserializeObject([Text.Encoding]::UTF8.GetString($bytes))
  $entries = $raw.Values |
    Where-Object { $_.IsReleased[0] } |
    Sort-Object { [int64]$_.Id }
  $lines = foreach ($entry in $entries) { & $fields $entry | ConvertTo-Json -Compress }
  $json = "[`n" + ($lines -join ",`n") + "`n]`n"
  [IO.File]::WriteAllText((Join-Path $root "$name.json"), $json, (New-Object Text.UTF8Encoding($false)))

  if (-not $iconDir) { "${name}: $($entries.Count) entries"; return }
  $dir = Join-Path $root $iconDir
  New-Item -ItemType Directory -Force $dir | Out-Null
  $fetched = 0
  foreach ($icon in ($entries.Icon | Sort-Object -Unique)) {
    $path = Join-Path $dir "$icon.webp"
    if (Test-Path $path) { continue }
    curl.exe -sSf -o $path "$iconUrl/$icon.webp"
    if ($LASTEXITCODE -ne 0) { throw "failed to fetch icon $icon" }
    $fetched++
  }
  "${name}: $($entries.Count) entries, $fetched new icons"
}

Update-List 'items' 'icons' 'https://schaledb.com/images/item/icon' {
  param($item)
  [ordered]@{
    id = [int64]$item.Id
    name = ($item.Name -replace '[\r\n]+', '')
    icon = $item.Icon
    category = $item.Category
    rarity = $item.Rarity
  }
}

Update-List 'equipment' 'equipment-icons' 'https://schaledb.com/images/equipment/icon' {
  param($equipment)
  [ordered]@{
    id = [int64]$equipment.Id
    name = ($equipment.Name -replace '[\r\n]+', '')
    icon = $equipment.Icon
    category = $equipment.Category
    rarity = $equipment.Rarity
    tier = [int]$equipment.Tier
  }
}

# Students are told apart by name, so no pictures are needed.
Update-List 'students' $null $null {
  param($student)
  [ordered]@{
    id = [int64]$student.Id
    name = ($student.Name -replace '[
]+', '')
    equipment = @($student.Equipment)
  }
}
