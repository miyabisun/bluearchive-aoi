# Raises the app's version by a patch in the two places a release names it (Cargo.toml and
# Cargo.lock), as agent-tools' bump.sh does by hand, and prints the new version. Stops,
# changing nothing, when either place does not hold the version Cargo.toml has.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$manifest = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'))
$found = [regex]::Match($manifest, '\[package\]\r?\nname = "bluearchive-aoi"\r?\nversion = "([0-9.]+)"')
if (-not $found.Success) { throw "Cargo.toml names no version of bluearchive-aoi" }
$old = $found.Groups[1].Value
$parts = $old.Split('.')
$new = '{0}.{1}.{2}' -f $parts[0], $parts[1], ([int]$parts[2] + 1)
$v = [regex]::Escape($old)
$places = @(
  @('Cargo.toml', "(\[package\]\r?\nname = `"bluearchive-aoi`"\r?\nversion = `")$v`"", 1),
  @('Cargo.lock', "(name = `"bluearchive-aoi`"\r?\nversion = `")$v`"", 1)
)
$texts = @{}
foreach ($place in $places) {
  $path, $pattern, $count = $place
  $text = [IO.File]::ReadAllText((Join-Path $root $path))
  $found = [regex]::Matches($text, $pattern).Count
  if ($found -ne $count) { throw "$path names $old $found times, not $count" }
  $texts[$path] = [regex]::Replace($text, $pattern, "`${1}$new`"")
}
$utf8 = New-Object Text.UTF8Encoding $false
foreach ($path in $texts.Keys) { [IO.File]::WriteAllText((Join-Path $root $path), $texts[$path], $utf8) }
$new
