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

$summary = ''
$transcript = $data.transcript_path
if ($transcript -and (Test-Path $transcript)) {
  $lines = Get-Content -Path $transcript -Encoding utf8
  for ($i = $lines.Length - 1; $i -ge 0; $i--) {
    $line = $lines[$i]
    if (-not $line) { continue }
    try { $m = $line | ConvertFrom-Json } catch { continue }
    if ($m.type -ne 'assistant') { continue }
    $content = $m.message.content
    if (-not $content) { continue }
    $textParts = @()
    foreach ($c in $content) {
      if ($c.type -eq 'text' -and $c.text) { $textParts += $c.text }
    }
    if ($textParts.Count -gt 0) {
      $summary = ($textParts -join "`r`n")
      break
    }
  }
}

$ts = Get-Date -Format 'HH:mm:ss'
$utf8 = New-Object System.Text.UTF8Encoding($false)

if ($summary) {
  if ($summary.Length -gt 1200) {
    $summary = $summary.Substring(0, 1200) + "`r`n[truncated]"
  }
  $entry = "`r`n### RESPONSE $ts`r`n`r`n$summary`r`n"
} else {
  $entry = "`r`n### TURN END $ts (no text response)`r`n"
}

[IO.File]::AppendAllText($file, $entry, $utf8)
