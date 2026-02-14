Get-ChildItem -Recurse -Force -File |
  Where-Object {
    $_.Name -eq 'NUL' -or $_.Name -match 'tempwarn' -or [regex]::IsMatch($_.Name,'[^\u0020-\u007E]')
  } | Select-Object FullName
Write-Host "Remove listed files and retry 'git add -A'."
