# Refreshes the SchaleDB snapshot the recognizer uses: the JP items that are
# released in Japan, sorted by Id (the game's default list order), and their
# icons. Tests and the app read only this snapshot, never the network.
#   powershell -File tools/schaledb/update.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..\..\src-tauri\data\schaledb'
$iconDir = Join-Path $root 'icons'
New-Item -ItemType Directory -Force $iconDir | Out-Null

# The response has no charset, which Windows PowerShell would read as Latin-1.
$bytes = (Invoke-WebRequest -UseBasicParsing 'https://schaledb.com/data/jp/items.json').RawContentStream.ToArray()
$raw = [Text.Encoding]::UTF8.GetString($bytes) | ConvertFrom-Json
$items = $raw.PSObject.Properties.Value |
  Where-Object { $_.IsReleased[0] } |
  Sort-Object { [int64]$_.Id }

# One item per line keeps the snapshot diffable when SchaleDB adds items.
$lines = foreach ($item in $items) {
  [ordered]@{
    id = [int64]$item.Id
    name = ($item.Name -replace '[\r\n]+', '')
    icon = $item.Icon
    category = $item.Category
    rarity = $item.Rarity
  } | ConvertTo-Json -Compress
}
$json = "[`n" + ($lines -join ",`n") + "`n]`n"
[IO.File]::WriteAllText((Join-Path $root 'items.json'), $json, (New-Object Text.UTF8Encoding($false)))

$fetched = 0
foreach ($icon in ($items.Icon | Sort-Object -Unique)) {
  $path = Join-Path $iconDir "$icon.webp"
  if (Test-Path $path) { continue }
  curl.exe -sSf -o $path "https://schaledb.com/images/item/icon/$icon.webp"
  if ($LASTEXITCODE -ne 0) { throw "failed to fetch icon $icon" }
  $fetched++
}
"$($items.Count) items, $fetched new icons"
