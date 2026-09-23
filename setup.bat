@echo off
chcp 65001 >nul
title Anon Kurulumu (Windows)
echo ===================================================
echo   Anon - Windows Icin Global PATH Kurulumu
echo ===================================================
echo.

wsl --status >nul 2>&1
if %errorLevel% neq 0 (
    echo [-] UYARI: WSL sisteminizde kurulu degil.
    echo [*] Ubuntu (WSL) otomatik olarak kuruluyor... Lutfen bekleyin.
    wsl --install -d Ubuntu
    echo [*] Kurulum bittikten sonra bilgisayarinizi yeniden baslatmaniz veya terminali kapatip acmaniz gerekebilir.
    pause
    exit /b 0
)

set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

echo [*] Dizin Kisisel PATH degiskenine ekleniyor...
powershell -Command "$path = [Environment]::GetEnvironmentVariable('PATH', 'User'); if ($path -notmatch [regex]::Escape('%SCRIPT_DIR%')) { [Environment]::SetEnvironmentVariable('PATH', $path + ';%SCRIPT_DIR%', 'User') }"

echo [*] WSL icin gerekli paketler yukleniyor (Ilk kurulumsa 1-2 dakika surebilir)...
wsl -u root -- bash -c "apt-get update && apt-get install -y make tar tor curl python3 python3-scapy network-manager obfs4proxy proxychains4 rfkill secure-delete iptables macchanger"

echo [*] Kurulum WSL (Linux Alt Sistemi) uzerine yapiliyor...
wsl -u root -- make install

echo.
echo [SUCCESS] Kurulum basariyla tamamlandi!
echo [SUCCESS] Yeni bir CMD acarak sadece "anon" yazip menuye erisebilirsiniz.
echo.
pause
