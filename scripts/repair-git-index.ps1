# backup + rebuild Git index on Windows
Copy-Item -ErrorAction SilentlyContinue .git\index .git\index.bak
Remove-Item -Force -ErrorAction SilentlyContinue .git\index
git reset --mixed
Write-Host "Index rebuilt. Run: git add -A && git commit -m '...' && git push"
