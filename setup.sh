#!/bin/bash
# Anon - Setup Script for Linux

if [ "$EUID" -ne 0 ]; then
  echo "[-] Lutfen root yetkisi ile calistirin: sudo ./setup.sh"
  exit 1
fi

echo "[*] Kurulum baslatiliyor..."
apt-get update
apt-get install -y \
    cargo rustc make tar tor curl iproute2 iptables macchanger \
    obfs4proxy snowflake-client dnscrypt-proxy proxychains4 \
    rfkill secure-delete i2pd firejail network-manager \
    mat2 libimage-exiftool-perl libnotify-bin psmisc unzip \
    apparmor-utils dmidecode iputils-ping ca-certificates

if make install; then
    echo "[+] Kurulum basarili!"
    echo "[+] Artik terminalin herhangi bir yerinden 'sudo anon' komutunu calistirabilirsiniz."
else
    echo "[-] Kurulum sirasinda bir hata olustu."
    exit 1
fi
