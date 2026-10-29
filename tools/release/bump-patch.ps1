# Raises the app's version by a patch in the five places a release names it
# (package.json, package-lock.json twice, src-tauri/Cargo.toml and Cargo.lock),
# as agent-tools' bump.sh does by hand, and prints the new version. Stops,
# changing nothing, when any place does not hold the version package.json has.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$old = (Get-Content (Join-Path $root 'package.json') -Raw | ConvertFrom-Json).version
$parts = $old.Split('.')
$new = '{0}.{1}.{2}' -f $parts[0], $parts[1], ([int]$parts[2] + 1)
$v = [regex]::Escape($old)
$name = '"name": "bluearchive-aoi",\r?\n\s*"version": "'
$places = @(
  @('package.json', "(?m)^(  `"version`": `")$v`"", 1),
  @('package-lock.json', "($name)$v`"", 2),
  @('src-tauri/Cargo.toml', "(\[package\]\r?\nname = `"bluearchive-aoi`"\r?\nversion = `")$v`"", 1),
  @('src-tauri/Cargo.lock', "(name = `"bluearchive-aoi`"\r?\nversion = `")$v`"", 1)
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
