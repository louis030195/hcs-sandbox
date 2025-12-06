@echo off
echo === HyperV-Kube Setup ===
echo.
echo This will:
echo   1. Enable Hyper-V (if needed)
echo   2. Download Windows 11 Dev VM (~20GB)
echo   3. Create a test VM
echo.
echo Press any key to start (will request Admin rights)...
pause > nul

powershell -Command "Start-Process powershell -Verb RunAs -ArgumentList '-ExecutionPolicy Bypass -File \"%~dp0scripts\quick-setup.ps1\"'"
