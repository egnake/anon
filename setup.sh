#!/bin/bash
# Anon - Setup Script for Linux

if [ "$EUID" -ne 0 ]; then
  echo "[-] Lutfen root yetkisi ile calistirin: sudo ./setup.sh"
  exit 1
fi

echo "[*] Kurulum baslatiliyor..."
make install

if [ $? -eq 0 ]; then
    echo "[+] Kurulum basarili!"
    echo "[+] Artik terminalin herhangi bir yerinden 'sudo anon' komutunu calistirabilirsiniz."
else
    echo "[-] Kurulum sirasinda bir hata olustu."
    exit 1
fi
