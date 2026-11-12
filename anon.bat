@echo off
:: Anon Windows Wrapper
:: Converts Windows drive paths (e.g. D:\folder\file.jpg) to WSL /mnt paths and runs anon as root
wsl -u root -- bash -c "args=(); for a in \"$@\"; do if [[ \"$a\" =~ ^[A-Za-z]:[/\\\\] ]]; then args+=(\"$(wslpath -u \"$a\" 2>/dev/null || printf '%s' \"$a\")\"); else args+=(\"$a\"); fi; done; exec anon \"${args[@]}\"" _ %*
