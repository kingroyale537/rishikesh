# ==============================================================================
# Rishikesh Language Windows PowerShell Installer
# Usage: iwr -useb https://rishikesh.lang/install.ps1 | iex
# ==============================================================================

Write-Host "  ____  _     _     _ _              _     " -ForegroundColor Cyan
Write-Host " |  _ \(_)___| |__ (_) | _____  ___ | |__  " -ForegroundColor Cyan
Write-Host " | |_) | / __| '_ \| | |/ / _ \/ __|| '_ \ " -ForegroundColor Cyan
Write-Host " |  _ <| \__ \ | | | |   <  __/\__ \| | | |" -ForegroundColor Cyan
Write-Host " |_| \_\_|___/_| |_|_|_|\_\___||___/|_| |_|" -ForegroundColor Cyan
Write-Host "`nThe Universal Programming Language for AI, Systems, and Cloud`n" -ForegroundColor White

$InstallDir = "$env:USERPROFILE\.rishi\bin"
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
}

Write-Host "✓ Installation directory configured: $InstallDir" -ForegroundColor Green

# Add to user PATH if not present
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", [EnvironmentVariableTarget]::User)
    Write-Host "✓ Added $InstallDir to User PATH environment variable" -ForegroundColor Green
}

Write-Host "`n🎉 Rishikesh Language installed successfully on Windows!" -ForegroundColor Green
Write-Host "Restart your PowerShell terminal and run:"
Write-Host "  rishi --help" -ForegroundColor Yellow
Write-Host "  rishi repl" -ForegroundColor Yellow
