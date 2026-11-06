#!/bin/bash

echo -e "\e[1;36m=================================================\e[0m"
echo -e "\e[1;36m      Anon - Whitebox & Blackbox Test Suite      \e[0m"
echo -e "\e[1;36m=================================================\e[0m\n"

# --- Setup Mock Environment ---
export MOCK_DIR=$(mktemp -d)
export PATH="$MOCK_DIR:$PATH"
export TEST_LOGS="$MOCK_DIR/test.log"
touch "$TEST_LOGS"

# Function to mock system commands to prevent them from executing for real
mock_cmd() {
    local cmd=$1
    echo "#!/bin/bash" > "$MOCK_DIR/$cmd"
    echo "echo \"$cmd \$@\" >> $TEST_LOGS" >> "$MOCK_DIR/$cmd"
    
    # Specific mock behaviors
    if [ "$cmd" = "curl" ]; then
        echo "echo '\"IsTor\":true'" >> "$MOCK_DIR/$cmd"
    elif [ "$cmd" = "ip" ]; then
        # Mocking interfaces
        echo "if [[ \"\$*\" == *\"-o link show\"* ]]; then echo '1: lo: <LOOPBACK>'; echo '2: eth0: <BROADCAST,MULTICAST>'; echo '3: wlan0: <BROADCAST,MULTICAST>'; fi" >> "$MOCK_DIR/$cmd"
    elif [ "$cmd" = "dmidecode" ]; then
        echo "echo 'Mocked Manufacturer'" >> "$MOCK_DIR/$cmd"
    elif [ "$cmd" = "nmcli" ]; then
        echo "echo 'wlan0 wifi'" >> "$MOCK_DIR/$cmd"
    elif [ "$cmd" = "iptables-restore" ] || [ "$cmd" = "ip6tables-restore" ]; then
        echo "while read -r line; do echo \"$cmd: \$line\" >> $TEST_LOGS; done" >> "$MOCK_DIR/$cmd"
    fi
    chmod +x "$MOCK_DIR/$cmd"
}

mock_cmd iptables
mock_cmd ip6tables
mock_cmd iptables-save
mock_cmd ip6tables-save
mock_cmd iptables-restore
mock_cmd ip6tables-restore
mock_cmd sysctl
mock_cmd systemctl
mock_cmd curl
mock_cmd timedatectl
mock_cmd dmidecode
mock_cmd nmcli
mock_cmd macchanger
mock_cmd obfs4proxy
mock_cmd shred
mock_cmd ip
mock_cmd hostname
mock_cmd hostnamectl
mock_cmd killall

# Global required variables
export BASE_DIR="$(pwd)"
export SRCDIR="$(pwd)/assets"
export BACKUPDIR="$(pwd)/backups"
mkdir -p "$BACKUPDIR"

# Bypass root check in a temp copy of anon
cp anon anon_test
sed -i 's/\[\[ "$EUID" -ne 0 \]\]/\[\[ "0" -ne 0 \]\]/g' anon_test
chmod +x anon_test

source assets/sources/config

echo -e "\e[1;33m[*] Running Whitebox Tests (Internal Logic Verification)...\e[0m"
FAIL_COUNT=0
PASS_COUNT=0

assert_log() {
    if grep -q "$1" "$TEST_LOGS"; then
        echo -e "  \e[32m[PASS]\e[0m $2"
        ((PASS_COUNT++))
    else
        echo -e "  \e[31m[FAIL]\e[0m $2 (Expected to find: $1 in logs)"
        ((FAIL_COUNT++))
    fi
}

# --- 1. os_obfuscation ---
echo -e "\n\e[1;34m-> Module: os_obfuscation\e[0m"
source assets/scripts/os_obfuscation
os_obfuscation_status="Disable"
start_os_obfuscation >/dev/null
assert_log "sysctl -w net.ipv4.ip_default_ttl=128" "Sets TTL to 128 (Windows fingerprint)"
assert_log "sysctl -w net.ipv4.tcp_window_scaling=0" "Disables TCP window scaling"

# --- 2. ip_changer ---
echo -e "\n\e[1;34m-> Module: ip_changer\e[0m"
source assets/scripts/ip_changer
ip_changer_status="Disable"
dns_changer_status="Disable"
mkdir -p /etc/tor 2>/dev/null || true
echo "dummy" > /etc/tor/torrc 2>/dev/null || true
export TORRC="/etc/tor/torrc"
start_ip_changer >/dev/null 2>&1 || true
assert_log "iptables -t nat -A OUTPUT -p tcp -m tcp --tcp-flags FIN,SYN,RST,ACK SYN -j REDIRECT --to-ports 9040" "Configures Transparent Proxy (REDIRECT)"
assert_log "iptables -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP" "Drops direct LAN DNS attempts to prevent leaks"
assert_log "systemctl start tor.service" "Triggers tor.service start"
assert_log "check.torproject.org/api/ip" "Executes automated Tor leak test"

# --- 3. mac_changer ---
echo -e "\n\e[1;34m-> Module: mac_changer\e[0m"
source assets/scripts/mac_changer
mac_changer_status="Disable"
start_mac_changer >/dev/null 2>&1 || true
assert_log "ip link set wlan0 down" "Takes network interface down safely"
assert_log "ip link set wlan0 address" "Assigns new authentic vendor-spoofed MAC address"

# --- 4. kill_switch ---
echo -e "\n\e[1;34m-> Module: kill_switch\e[0m"
source assets/scripts/kill_switch
kill_switch_status="Disable"
start_kill_switch >/dev/null 2>&1 || true
assert_log "iptables-restore: :OUTPUT DROP" "Sets default OUTPUT policy to DROP atomically"
assert_log "iptables-restore: -A OUTPUT -o lo -j ACCEPT" "Ensures localhost traffic remains allowed during lockdown"
assert_log "iptables-restore: -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP" "Blocks LAN DNS leakage in kill switch"
assert_log "ip6tables-restore: :OUTPUT DROP" "Enforces complete IPv6 kill switch lockdown"

# --- 5. hostname_changer ---
echo -e "\n\e[1;34m-> Module: hostname_changer\e[0m"
source assets/scripts/hostname_changer
hostname_changer_status="Disable"
start_hostname_changer >/dev/null 2>&1 || true
assert_log "hostname " "Applies spoofed hostname live to running kernel"

# --- 6. anti_cold_boot ---
echo -e "\n\e[1;34m-> Module: anti_cold_boot\e[0m"
source assets/scripts/anti_cold_boot
anti_cold_boot_status="Disable"
start_anti_cold_boot >/dev/null 2>&1 || true
assert_log "sysctl -w fs.suid_dumpable=0" "Locks suid core-dumps against forensic extraction"
assert_log "sysctl -w kernel.core_pattern=|/bin/false" "Disables kernel crash dumps to disk"

# --- 7. emergency_panic ---
echo -e "\n\e[1;34m-> Module: emergency_panic\e[0m"
source assets/scripts/emergency_panic
trigger_panic >/dev/null 2>&1 || true
assert_log "ip link set eth0 down" "Severs Ethernet interfaces immediately"
assert_log "killall -9 firefox" "Terminates browser and sensitive processes"
assert_log "iptables -P OUTPUT DROP" "Locks all outbound traffic permanently"

echo -e "\n\e[1;33m[*] Running Blackbox Tests (End-to-End Execution)...\e[0m"
# Clear logs for blackbox
> "$TEST_LOGS"

echo -e "\n\e[1;34m-> Feature: CLI Argument (--status)\e[0m"
./anon_test --status > blackbox_out.txt 2>&1
if grep -q "ANON SYSTEM STATUS" blackbox_out.txt; then
    echo -e "  \e[32m[PASS]\e[0m System status dashboard prints successfully"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m System status failed"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--help)\e[0m"
./anon_test --help > blackbox_help.txt 2>&1
if grep -q "\-\-panic" blackbox_help.txt; then
    echo -e "  \e[32m[PASS]\e[0m CLI help displays --panic option"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m CLI help missing --panic option"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;36m=================================================\e[0m"
echo -e "Test Results: \e[32m$PASS_COUNT Passed\e[0m | \e[31m$FAIL_COUNT Failed\e[0m"
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "\e[1;32mALL TESTS PASSED SUCCESSFULLY! The architecture is solid.\e[0m"
else
    echo -e "\e[1;31mSOME TESTS FAILED! Please check the logs.\e[0m"
fi
echo -e "\e[1;36m=================================================\e[0m"

# Cleanup
rm -rf "$MOCK_DIR"
rm -f anon_test blackbox_out.txt blackbox_help.txt
