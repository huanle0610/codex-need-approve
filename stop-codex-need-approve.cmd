@echo off
for /f "tokens=2" %%P in ('tasklist /fi "imagename eq codex-need-approve.exe" /fo list ^| findstr /b "PID:"') do taskkill /pid %%P /f >nul 2>nul
echo Codex Need Approve stopped.
