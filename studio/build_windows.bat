@echo off
rem Сборка DDS Studio для Windows (обёртка над build_windows.ps1)
rem   build_windows.bat            - DDS-Studio-vX.Y.Z-windows-x64.zip
rem   build_windows.bat -WithServer
chcp 65001 >nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build_windows.ps1" %*
exit /b %ERRORLEVEL%
