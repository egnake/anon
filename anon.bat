@echo off
:: Anon Windows Wrapper
:: Executes the Linux bash script via WSL
wsl -u root -- anon %*
