$ErrorActionPreference = 'SilentlyContinue'

$cwd = $env:CLAUDE_PROJECT_DIR
if (-not $cwd) {
  try {
    $stdin = [Console]::In.ReadToEnd()
    if ($stdin) { $cwd = ($stdin | ConvertFrom-Json).cwd }
  } catch {}
}
if (-not $cwd) { $cwd = (Get-Location).Path }

$istori = Join-Path $cwd 'istori'
if (-not (Test-Path $istori)) {
  New-Item -ItemType Directory -Path $istori -Force | Out-Null
}

$latest = Get-ChildItem -Path $istori -Filter '*.md' -File -ErrorAction SilentlyContinue |
  Where-Object { $_.Name -ne 'README.md' } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 1

$ts = Get-Date -Format 'yyyy-MM-dd_HH-mm'
$file = Join-Path $istori "$ts.md"
$i = 1
while (Test-Path $file) {
  $file = Join-Path $istori ("{0}_{1}.md" -f $ts, $i)
  $i++
}

$utf8 = New-Object System.Text.UTF8Encoding($false)
$header = "# Session $(Get-Date -Format 'yyyy-MM-dd HH:mm')`r`n`r`n"
[IO.File]::WriteAllText($file, $header, $utf8)

$stateDir = Join-Path $cwd '.claude'
if (-not (Test-Path $stateDir)) {
  New-Item -ItemType Directory -Path $stateDir -Force | Out-Null
}
[IO.File]::WriteAllText((Join-Path $stateDir '.istori-current'), $file, $utf8)

if ($latest) {
  Write-Output ("## Previous session - istori/" + $latest.Name)
  Write-Output ''
  Get-Content $latest.FullName -Raw -Encoding utf8 | Write-Output
  Write-Output ''
  Write-Output '---'
}
Write-Output ('Current session file: istori/' + [IO.Path]::GetFileName($file))
