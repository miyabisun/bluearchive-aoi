# Writes `stages.json`: what the daily stages drop, for the work tab. Normal
# missions with the blueprints each run gives on average and its AP; each
# bounty with the items it drops; each scrimmage with the equipment it drops;
# and the stages' names. Called by update.ps1.
#   powershell -File tools/schaledb/stages.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..\..\src-tauri\data\schaledb'
Add-Type -AssemblyName System.Web.Extensions

function Get-Json($url) {
  $bytes = (Invoke-WebRequest -UseBasicParsing $url).RawContentStream.ToArray()
  $parser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
  $parser.MaxJsonLength = [int]::MaxValue
  $parser.DeserializeObject([Text.Encoding]::UTF8.GetString($bytes))
}

# A field of a reward, or `$default` where it has none.
function Field($reward, $name, $default) {
  if ($reward.ContainsKey($name)) { $reward[$name] } else { $default }
}

$stages = (Get-Json 'https://schaledb.com/data/jp/stages.json').Values
$groups = Get-Json 'https://schaledb.com/data/groups.min.json'
$names = (Get-Json 'https://schaledb.com/data/jp/localization.json')['StageType']

# What a run gives on average of each `$type` (Item, Equipment): the rewards
# that come every run, gacha groups opened by their chances.
function Drops($stage, $type) {
  $out = @{}
  foreach ($r in $stage['Rewards']) {
    if ($r.ContainsKey('RewardType')) { continue }
    $times = [double](Field $r 'Chance' 1) * [double](Field $r 'Amount' 1)
    $items = if ($r['Type'] -eq 'GachaGroup') { $groups["$($r['Id'])"]['Items'] } else { @($r) }
    $each = if ($r['Type'] -eq 'GachaGroup') { $times } else { 1 }
    foreach ($i in $items) {
      if ($i['Type'] -ne $type) { continue }
      $amount = ([double](Field $i 'AmountMin' 1) + [double](Field $i 'AmountMax' 1)) / 2
      $chance = if ($r['Type'] -eq 'GachaGroup') { [double](Field $i 'Chance' 1) } else { $times }
      $id = [int64]$i['Id']
      $out[$id] = (Field $out $id 0) + $each * $chance * $amount
    }
  }
  $out
}

# Normal missions: the blueprints (ids from 100000) a run gives.
$missions = foreach ($s in ($stages | Where-Object { $_['Category'] -eq 'Campaign' -and $_['Difficulty'] -eq 0 } |
    Sort-Object { [int64]$_['Id'] })) {
  $drops = Drops $s 'Equipment'
  $pairs = @(foreach ($id in ($drops.Keys | Where-Object { $_ -ge 100000 } | Sort-Object)) {
    , @([int64]$id, [math]::Round($drops[$id], 4))
  })
  if ($pairs.Count -eq 0) { continue }
  $ap = ($s['EntryCost'] | Where-Object { $_[0] -eq 5 } | Select-Object -First 1)[1]
  [ordered]@{ name = "$($s['Area'])-$($s['Stage'])"; ap = [int]$ap; drops = $pairs }
}

# Each bounty and scrimmage: what any of its stages drops of a kind.
function Places($category, $type) {
  $byType = [ordered]@{}
  foreach ($s in ($stages | Where-Object { $_['Category'] -eq $category } | Sort-Object { [int64]$_['Id'] })) {
    $t = $s['Type']
    if (-not $byType.Contains($t)) { $byType[$t] = @{} }
    foreach ($id in (Drops $s $type).Keys) { $byType[$t][[int64]$id] = $true }
  }
  foreach ($t in $byType.Keys) {
    [ordered]@{ type = $t; name = $names[$t]; drops = @($byType[$t].Keys | Sort-Object) }
  }
}

$out = [ordered]@{
  missions = @($missions)
  bounties = @(Places 'Bounty' 'Item')
  scrimmages = @(Places 'SchoolDungeon' 'Equipment')
  commissions = [ordered]@{ FindGift = $names['FindGift']; Blood = $names['Blood'] }
}
$text = $out | ConvertTo-Json -Depth 6 -Compress
[IO.File]::WriteAllText((Join-Path $root 'stages.json'), $text + "`n", (New-Object Text.UTF8Encoding($false)))
Write-Host "stages: $(@($missions).Count) missions"
