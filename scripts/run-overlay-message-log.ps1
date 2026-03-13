# Run overlay Win32 message log: you trigger start in the app, hold the hotkey, then we open the log.
# Requires: Kalam running (e.g. npm run tauri dev), overlay visible at least once so the window exists.

$logDir = if ($env:USERPROFILE) { Join-Path $env:USERPROFILE ".kalam" } else { Join-Path $env:HOME ".kalam" }
$logPath = Join-Path $logDir "overlay-messages.log"

Write-Host "=== Overlay message log (Win32) ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "1. Open the Kalam MAIN window (not just the tray)."
Write-Host "2. Open DevTools: Right-click in the window -> Inspect, or press F12 if enabled."
Write-Host "3. In the Console tab, paste and run:"
Write-Host ""
Write-Host "   await __TAURI_INTERNALS__.invoke('start_overlay_message_log_for_seconds', { seconds: 15 })" -ForegroundColor Yellow
Write-Host ""
Write-Host "4. Immediately HOLD your dictation hotkey for ~15 seconds (overlay will show Listening/Recording)."
Write-Host "5. When 15 seconds have passed, logging stops automatically."
Write-Host "6. Log file: $logPath"
Write-Host ""
Write-Host "Press Enter when you have finished (after the 15 seconds) to open the log file..."
$null = Read-Host

if (Test-Path $logPath) {
    Get-Content $logPath
    Write-Host ""
    Write-Host "Opening log in default editor..."
    Start-Process $logPath
} else {
    Write-Host "Log file not found. Did you run the invoke and hold the hotkey?" -ForegroundColor Red
    Write-Host "Path: $logPath"
}
