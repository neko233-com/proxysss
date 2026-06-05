$pidFile = Join-Path $PSScriptRoot ".services.pids"

if (Test-Path $pidFile) {
    Get-Content $pidFile | ForEach-Object {
        if ($_ -match '^(.+)=(\d+)$') {
            Stop-Process -Id ([int]$Matches[2]) -Force -ErrorAction SilentlyContinue
            Write-Host "stopped $($Matches[1]) pid=$($Matches[2])"
        }
    }
    Remove-Item $pidFile -Force
}

Write-Host "backend services stopped"
