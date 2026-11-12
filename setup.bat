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
    echo [*] Ubuntu ^(WSL^) otomatik olarak kuruluyor... Lutfen bekleyin.
    wsl --install -d Ubuntu
    echo [*] Kurulum bittikten sonra bilgisayarinizi yeniden baslatmaniz gerekebilir.
    pause
    exit /b 0
)

set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

echo [*] Dizin Kisisel PATH degiskenine ekleniyor...
set "ANON_WIN_DIR=%SCRIPT_DIR%"
powershell -NoProfile -Command "$dir = $env:ANON_WIN_DIR; $p = [Environment]::GetEnvironmentVariable('PATH', 'User'); if (-not $p) { $p = '' }; $parts = $p -split ';' | Where-Object { $_ -ne '' }; if ($parts -notcontains $dir) { [Environment]::SetEnvironmentVariable('PATH', (($parts + $dir) -join ';'), 'User') }"

echo [*] WSL systemd kontrol ediliyor (/etc/wsl.conf)...
wsl -u root -- bash -c "if [ ! -d /run/systemd/system ]; then if ! grep -q 'systemd=true' /etc/wsl.conf 2>/dev/null; then printf '[boot]\nsystemd=true\n' >> /etc/wsl.conf; echo '[!] /etc/wsl.conf icine systemd=true eklendi (wsl --shutdown sonrasi aktif olur).'; fi; fi"

echo [*] WSL icin gerekli paketler yukleniyor (Ilk kurulumsa 1-2 dakika surebilir)...
wsl -u root -- bash -c "apt-get update && apt-get install -y make tar tor curl iproute2 iptables macchanger obfs4proxy snowflake-client dnscrypt-proxy proxychains4 rfkill secure-delete i2pd firejail network-manager mat2 libimage-exiftool-perl libnotify-bin psmisc unzip apparmor-utils dmidecode iputils-ping ca-certificates"

echo [*] Kurulum WSL uzerine yapiliyor...
wsl -u root -- make install

echo.
echo [SUCCESS] Kurulum basariyla tamamlandi!
echo [SUCCESS] Yeni bir terminal acarak "anon" komutunu calistirabilirsiniz.
echo.
pause
