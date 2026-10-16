# Refreshes the SchaleDB snapshot the recognizer uses: the JP items, equipment
# and students that are released in Japan, each sorted by Id (the game's
# default list order), and the item and equipment icons; and the growth tables
# the training plan uses (`growth.json`), the daily stages (`stages.json`, by
# stages.ps1) and the choice boxes (`boxes.json`, by boxes.ps1). Tests and the
# app read only this snapshot, never the network.
#   powershell -File tools/schaledb/update.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $PSScriptRoot '..\..\src-tauri\data\schaledb'
Add-Type -AssemblyName System.Web.Extensions

# The JSON at `$url`, read as UTF-8 whatever the response says.
function Get-Json($url) {
  # The response has no charset, which Windows PowerShell would read as Latin-1.
  $bytes = (Invoke-WebRequest -UseBasicParsing $url).RawContentStream.ToArray()
  # ConvertFrom-Json refuses keys that differ only in case, which students.json has.
  $parser = New-Object System.Web.Script.Serialization.JavaScriptSerializer
  $parser.MaxJsonLength = [int]::MaxValue
  $parser.DeserializeObject([Text.Encoding]::UTF8.GetString($bytes))
}

function Write-Json($name, $text) {
  [IO.File]::WriteAllText((Join-Path $root $name), $text, (New-Object Text.UTF8Encoding($false)))
}

# Writes `<name>.json` (one entry per line keeps the snapshot diffable when
# SchaleDB adds entries) and fetches the icons missing from `<iconDir>` (none
# without one). Returns the released entries as SchaleDB has them.
function Update-List($name, $iconDir, $iconUrl, $fields) {
  $entries = (Get-Json "https://schaledb.com/data/jp/$name.json").Values |
    Where-Object { $_.IsReleased[0] } |
    Sort-Object { [int64]$_.Id }
  $lines = foreach ($entry in $entries) { & $fields $entry | ConvertTo-Json -Compress -Depth 8 }
  Write-Json "$name.json" ("[`n" + ($lines -join ",`n") + "`n]`n")

  if ($iconDir) {
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
    Write-Host "${name}: $($entries.Count) entries, $fetched new icons"
  } else {
    Write-Host "${name}: $($entries.Count) entries"
  }
  , $entries
}

# Ids and amounts side by side as `[[id, amount], ...]`, one list per level.
function Pairs($ids, $amounts) {
  , @(for ($i = 0; $i -lt @($ids).Count; $i++) {
    , @(for ($j = 0; $j -lt @($ids[$i]).Count; $j++) { , @([int64]$ids[$i][$j], [int]$amounts[$i][$j]) })
  })
}

$items = Update-List 'items' 'icons' 'https://schaledb.com/images/item/icon' {
  param($item)
  $fields = [ordered]@{
    id = [int64]$item.Id
    name = ($item.Name -replace '[\r\n]+', '')
    icon = $item.Icon
    category = $item.Category
    rarity = $item.Rarity
  }
  # The kind within a category (Artifact, BookItem, CDItem), which the shops'
  # prices go by.
  if ($item.SubCategory) { $fields.subcategory = $item.SubCategory }
  $fields
}

# The currencies, for the shops' coins that are no item (the mastery
# certificate), with their icons beside the items'.
$currency = Get-Json 'https://schaledb.com/data/jp/currency.json' |
  Where-Object { $_.Icon } |
  Sort-Object { [int64]$_.Id }
$lines = foreach ($c in $currency) {
  [ordered]@{ id = [int64]$c.Id; name = $c.Name; icon = $c.Icon } | ConvertTo-Json -Compress
}
Write-Json 'currency.json' ("[`n" + ($lines -join ",`n") + "`n]`n")
foreach ($icon in ($currency.Icon | Sort-Object -Unique)) {
  $path = Join-Path $root "icons\$icon.webp"
  if (Test-Path $path) { continue }
  curl.exe -sSf -o $path "https://schaledb.com/images/item/icon/$icon.webp"
  if ($LASTEXITCODE -ne 0) { throw "failed to fetch icon $icon" }
}
Write-Host "currency: $($currency.Count) entries"

# Equipment also carries how it is made from the tier below (blueprints and
# credits) and how far it levels.
$equipment = Update-List 'equipment' 'equipment-icons' 'https://schaledb.com/images/equipment/icon' {
  param($equipment)
  $fields = [ordered]@{
    id = [int64]$equipment.Id
    name = ($equipment.Name -replace '[\r\n]+', '')
    icon = $equipment.Icon
    category = $equipment.Category
    rarity = $equipment.Rarity
    tier = [int]$equipment.Tier
    max_level = [int]$equipment.MaxLevel
  }
  if ($equipment.Recipe) {
    $fields.recipe = @(foreach ($r in $equipment.Recipe) { , @([int64]$r[0], [int]$r[1]) })
    $fields.recipe_cost = [int64]$equipment.RecipeCost
  }
  $fields
}

# Students are told apart by name, so no pictures are needed. Their skills'
# materials, level by level: EX from Lv2 to 5, the other skills from Lv2 to 9.
Update-List 'students' $null $null {
  param($student)
  [ordered]@{
    id = [int64]$student.Id
    name = ($student.Name -replace '[
]+', '')
    equipment = @($student.Equipment)
    school = $student.School
    # The weapon type (SG, SMG, AR, ...), which the unique weapon's parts suit.
    weapon = $student.WeaponType
    # How the student is had in JP: 0 always, 1 limited, 2 an event's, 3 fest.
    limited = [int]$student.IsLimited[0]
    ex = Pairs $student.SkillExMaterial $student.SkillExMaterialAmount
    skills = Pairs $student.SkillMaterial $student.SkillMaterialAmount
    # The ooparts (their first tier) the student's potential takes.
    potential = if ($student.PotentialMaterial) { [int64]$student.PotentialMaterial } else { $null }
  }
} | Out-Null

# Growth tables. SchaleDB no longer carries the level tables, so they come from
# the game's own tables as electricgoat/ba-data extracts them (JP).
$gameData = 'https://raw.githubusercontent.com/electricgoat/ba-data/jp/Excel'
$levels = (Get-Json "$gameData/CharacterLevelExcelTable.json").DataList
$gearLevels = (Get-Json "$gameData/EquipmentLevelExcelTable.json").DataList
$const = (Get-Json "$gameData/ConstCommonExcelTable.json").DataList[0]
$config = Get-Json 'https://schaledb.com/data/config.min.json'
$weaponLevels = (Get-Json "$gameData/CharacterWeaponLevelExcelTable.json").DataList
$weaponBonus = (Get-Json "$gameData/CharacterWeaponExpBonusExcelTable.json").DataList
# The exp from Lv1 to each level: 0 for Lv1, then the totals the table gives.
$toLevel = @(0) + @($levels | Sort-Object { $_.Level } | Select-Object -First ($levels.Count - 1) | ForEach-Object { [int64]$_.TotalExp })
$weaponToLevel = @(0) + @($weaponLevels | Sort-Object { $_.Level } | Select-Object -First ($weaponLevels.Count - 1) | ForEach-Object { [int64]$_.TotalExp })
$gearSorted = @($gearLevels | Sort-Object { $_.Level })
$gearToLevel = @(for ($t = 0; $t -lt 10; $t++) {
  , (@(0) + @($gearSorted | Select-Object -First ($gearSorted.Count - 1) | ForEach-Object { [int64]$_.TotalExp[$t] }))
})
$growth = [ordered]@{
  level_exp = $toLevel
  level_credits = [int]$const.CharacterLvUpCoefficient
  reports = @($items | Where-Object { $_.Category -eq 'CharacterExpGrowth' } | ForEach-Object { , @([int64]$_.Id, [int64]$_.ExpValue) })
  equipment_exp = $gearToLevel
  equipment_credits = [int]$const.EquipmentLvUpCoefficient
  orbs = @($equipment | Where-Object { $_.Category -eq 'Exp' } | ForEach-Object { , @([int64]$_.Id, [int64]$_.LevelUpFeedExp) })
  # The credits each skill level takes, as the archived SchaleDB site
  # (SchaleDB/SchaleDB js/common.js) had them: EX Lv2 to 5, the others Lv2 to 10.
  ex_credits = @(80000, 500000, 3000000, 10000000)
  skill_credits = @(5000, 7500, 60000, 90000, 300000, 450000, 1500000, 2400000, 4000000)
  # Raising a student's star, from 1 to 5, and the unique weapon's, from 1 to
  # 4: [elephs, credits] a step. The game's recipes (ba-data
  # RecipeIngredientExcelTable, the same for every student); the weapon's 4th
  # step takes 200 elephs as the Blue Archive wiki's Unique weapons page has it,
  # where the recipe still holds a placeholder of 1.
  star_up = @(@(30, 10000), @(80, 40000), @(100, 200000), @(120, 1000000))
  weapon_up = @(@(120, 1000000), @(180, 1500000), @(200, 2000000))
  # The unique weapon's exp from Lv1 to each level and the credits each exp costs.
  weapon_exp = $weaponToLevel
  weapon_credits = [int]$const.WeaponLvUpCoefficient
  # The unique weapon's parts (springs, hammers, barrels, firing pins): the exp each gives.
  weapon_parts = @($equipment | Where-Object { $_.Category -like 'WeaponExpGrowth*' } | ForEach-Object { , @([int64]$_.Id, [int64]$_.LevelUpFeedExp) })
  # Each weapon type and the parts' kind (but the firing pins, which suit all)
  # giving it more exp, and how much more (per 10,000).
  weapon_bonus = @(foreach ($row in $weaponBonus) {
    $kind = @('WeaponExpGrowthA', 'WeaponExpGrowthB', 'WeaponExpGrowthC') | Sort-Object { -[int]$row.$_ } | Select-Object -First 1
    , @($row.WeaponType, $kind, [int]$row.$kind)
  })
  # Each level of a potential (HP, attack or healing, 1 to 25): the tier of its
  # ooparts (0 the first) and how many.
  potential = @($config.PotentialMaterial | ForEach-Object { , @([int]$_.ArtifactGrade, [int]$_.ArtifactAmount) })
  # A skill's Lv10 also takes one Secret Technique Note.
  secret_note = [int64]($items | Where-Object { $_.Icon -eq 'item_icon_skillbook_ultimate' }).Id
}
Write-Json 'growth.json' (($growth | ConvertTo-Json -Compress -Depth 8) + "`n")
Write-Host "growth: $($toLevel.Count) levels, $($gearToLevel.Count) equipment tiers"

& (Join-Path $PSScriptRoot 'stages.ps1')
& (Join-Path $PSScriptRoot 'boxes.ps1')
