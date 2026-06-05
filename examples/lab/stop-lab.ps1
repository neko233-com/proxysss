$pidFile = Join-Path $PSScriptRoot ".lab.pids"

if (Test-Path $pidFile) {
    Get-Content $pidFile | ForEach-Object {
        if ($_ -match '^(.+)=(\d+)$') {
            $name = $Matches[1]
            $pid = [int]$Matches[2]
            Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
            Write-Host "stopped $name pid=$pid"
        }
    }
    Remove-Item $pidFile -Force
}

Get-Process proxysss -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "lab stopped"
