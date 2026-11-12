@echo off
:: Anon Windows Wrapper
:: Converts Windows drive paths (e.g. D:\folder\file.jpg) to WSL /mnt paths and runs anon as root
wsl -u root -e bash -c "IFS=; set -f; args=(); for a in $@; do if [[ $a =~ ^[A-Za-z]:[/\\] ]]; then if p=$(wslpath -u $a 2>/dev/null); then a=$p; fi; fi; args+=($a); done; exec env -u IFS anon ${args[@]}" _ %*
