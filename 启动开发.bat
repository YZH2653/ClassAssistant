@echo off
rem 双击启动开发版（自动启动 Vite 开发服务器）
cd /d "%~dp0"
npm run tauri dev
pause
