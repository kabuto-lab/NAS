$ErrorActionPreference = 'SilentlyContinue'
try {
  $stdin = [Console]::In.ReadToEnd()
  $data = $stdin | ConvertFrom-Json
} catch { exit 0 }

$cwd = $data.cwd
if (-not $cwd) { $cwd = $env:CLAUDE_PROJECT_DIR }
if (-not $cwd) { exit 0 }

$state = Join-Path $cwd '.claude\.istori-current'
if (-not (Test-Path $state)) { exit 0 }
$file = (Get-Content $state -Raw -Encoding utf8).Trim()
if (-not $file -or -not (Test-Path $file)) { exit 0 }

$ts = Get-Date -Format 'HH:mm:ss'
$entry = "`r`n## PROMPT $ts`r`n`r`n" + $data.prompt + "`r`n"
$utf8 = New-Object System.Text.UTF8Encoding($false)
[IO.File]::AppendAllText($file, $entry, $utf8)
