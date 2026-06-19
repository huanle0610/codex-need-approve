@echo off
set "APP_DIR=%~dp0"
set "EXE=%APP_DIR%codex-need-approve.exe"
set "PIDFILE=%APP_DIR%codex-need-approve.pid"
for /f "tokens=2" %%P in ('tasklist /fi "imagename eq codex-need-approve.exe" /fo list ^| findstr /b "PID:"') do taskkill /pid %%P /f >nul 2>nul
start "Codex Need Approve" /min "%EXE%"
timeout /t 1 /nobreak >nul
for /f "tokens=2" %%P in ('tasklist /fi "imagename eq codex-need-approve.exe" /fo list ^| findstr /b "PID:"') do echo %%P>"%PIDFILE%"
echo Codex Need Approve started.
