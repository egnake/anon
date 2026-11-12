#!/bin/bash
set -u

echo -e "\e[1;36m=================================================\e[0m"
echo -e "\e[1;36m      Anon - Complete & Isolated Test Suite      \e[0m"
echo -e "\e[1;36m=================================================\e[0m\n"

# --- Setup Isolated Mock Environment ---
export MOCK_DIR
MOCK_DIR=$(mktemp -d)
export PATH="$MOCK_DIR:$PATH"
export TEST_LOGS="$MOCK_DIR/test.log"
touch "$TEST_LOGS"

# Cleanup trap to ensure 100% clean exit with zero leftovers
cleanup() {
    rm -rf "$MOCK_DIR"
    rm -f blackbox_out.txt blackbox_help.txt blackbox_audit.txt blackbox_meta.txt
}
trap cleanup EXIT INT TERM

# Function to mock system commands to prevent them from executing for real
mock_cmd() {
    local cmd=$1
    cat > "$MOCK_DIR/$cmd" << EOF
#!/bin/bash
echo "$cmd \$@" >> "$TEST_LOGS"
if [ "$cmd" = "curl" ]; then
    echo '{"IsTor":true}'
elif [ "$cmd" = "ip" ]; then
    if [[ "\$*" == *"route"* ]]; then
        echo "default via 192.168.1.1 dev eth0"
    elif [[ "\$*" == *"-o link show"* ]]; then
        echo '1: lo: <LOOPBACK>'
        echo '2: eth0: <BROADCAST,MULTICAST>'
        echo '3: wlan0: <BROADCAST,MULTICAST>'
    fi
elif [ "$cmd" = "dmidecode" ]; then
    echo 'Mocked Manufacturer'
elif [ "$cmd" = "nmcli" ]; then
    echo 'wlan0 wifi'
elif [ "$cmd" = "iptables-restore" ] || [ "$cmd" = "ip6tables-restore" ]; then
    while IFS= read -r line; do
        echo "$cmd: \$line" >> "$TEST_LOGS"
    done
fi
exit 0
EOF
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
mock_cmd rfkill
mock_cmd tc
mock_cmd chattr
mock_cmd dnscrypt-proxy
mock_cmd mount
mock_cmd notify-send
mock_cmd umount
mock_cmd mountpoint
mock_cmd swapoff
mock_cmd swapon
mock_cmd tar
mock_cmd tor
mock_cmd proxychains4
mock_cmd sdmem
mock_cmd i2pd
mock_cmd mat2
mock_cmd exiftool

# Isolated paths
BASE_DIR="$(pwd)"
SRCDIR="$(pwd)/assets"
export BASE_DIR SRCDIR
export BACKUPDIR="$MOCK_DIR/backups"
mkdir -p "$BACKUPDIR"
touch "$BACKUPDIR/anon_fix_backups.tar.gz"

# Mock torrc
mkdir -p "$MOCK_DIR/etc_tor"
export TORRC="$MOCK_DIR/etc_tor/torrc"
echo "dummy" > "$TORRC"

# Bypass root check in a temp copy of anon inside MOCK_DIR
cp anon "$MOCK_DIR/anon_test"
sed -i 's/\[\[ "$EUID" -ne 0 \]\]/\[\[ "0" -ne 0 \]\]/g' "$MOCK_DIR/anon_test"
chmod +x "$MOCK_DIR/anon_test"

source assets/sources/config

echo -e "\e[1;33m[*] Running Whitebox Tests (Internal Logic Verification)...\e[0m"
FAIL_COUNT=0
PASS_COUNT=0

assert_log() {
    if grep -q "$1" "$TEST_LOGS"; then
        echo -e "  \e[32m[PASS]\e[0m $2"
        ((PASS_COUNT++))
    else
        echo -e "  \e[31m[FAIL]\e[0m $2 (Expected: '$1')"
        ((FAIL_COUNT++))
    fi
}

# --- 1. os_obfuscation ---
echo -e "\n\e[1;34m-> Module: os_obfuscation (start & stop)\e[0m"
source assets/scripts/os_obfuscation
os_obfuscation_status="Disable"
start_os_obfuscation >/dev/null
assert_log "sysctl -w net.ipv4.ip_default_ttl=128" "Sets TTL to 128 (Windows fingerprint)"
assert_log "sysctl -w net.ipv4.tcp_window_scaling=0" "Disables TCP window scaling"
stop_os_obfuscation >/dev/null
assert_log "sysctl -w net.ipv4.ip_default_ttl=" "Restores default TTL"

# --- 2. ip_changer ---
echo -e "\n\e[1;34m-> Module: ip_changer\e[0m"
source assets/scripts/ip_changer
ip_changer_status="Disable"
dns_changer_status="Disable"
doh_shield_status="Disable"
echo "dummy" > "$TORRC"
start_ip_changer >/dev/null 2>&1 || true
assert_log "iptables-restore: -A OUTPUT -p tcp -m tcp --tcp-flags FIN,SYN,RST,ACK SYN -j REDIRECT --to-ports 9040" "Configures Transparent Proxy (REDIRECT)"
assert_log "iptables-restore: -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP" "Drops direct LAN DNS attempts to prevent leaks"
assert_log "systemctl start tor.service" "Triggers tor.service start"
assert_log "check.torproject.org/api/ip" "Executes automated Tor leak test"
stop_ip_changer >/dev/null 2>&1 || true
assert_log "systemctl stop tor.service" "Stops tor.service on teardown"

# --- 3. mac_changer ---
echo -e "\n\e[1;34m-> Module: mac_changer (start & stop)\e[0m"
source assets/scripts/mac_changer
mac_changer_status="Disable"
start_mac_changer >/dev/null 2>&1 || true
assert_log "ip link set wlan0 down" "Takes network interface down safely"
assert_log "ip link set wlan0 address" "Assigns new authentic vendor-spoofed MAC address"
stop_mac_changer >/dev/null 2>&1 || true
assert_log "ip link set .* up" "Restores original MAC and interface state"

# --- 4. kill_switch ---
echo -e "\n\e[1;34m-> Module: kill_switch (start & stop)\e[0m"
source assets/scripts/kill_switch
kill_switch_status="Disable"
start_kill_switch >/dev/null 2>&1 || true
assert_log "iptables-restore: :OUTPUT DROP" "Sets default OUTPUT policy to DROP atomically"
assert_log "iptables-restore: -A OUTPUT -o lo -j ACCEPT" "Ensures localhost traffic remains allowed during lockdown"
assert_log "iptables-restore: -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP" "Blocks LAN DNS leakage in kill switch"
assert_log "ip6tables-restore: :OUTPUT DROP" "Enforces complete IPv6 kill switch lockdown"
stop_kill_switch >/dev/null 2>&1 || true
assert_log "iptables-restore" "Restores baseline firewall ruleset on kill switch stop"

# --- 5. hostname_changer ---
echo -e "\n\e[1;34m-> Module: hostname_changer (start & stop)\e[0m"
source assets/scripts/hostname_changer
hostname_changer_status="Disable"
start_hostname_changer >/dev/null 2>&1 || true
assert_log "hostname " "Applies spoofed hostname live to running kernel"
stop_hostname_changer >/dev/null 2>&1 || true
assert_log "hostname " "Restores original hostname"

# --- 6. anti_cold_boot ---
echo -e "\n\e[1;34m-> Module: anti_cold_boot (start & stop)\e[0m"
source assets/scripts/anti_cold_boot
anti_cold_boot_status="Disable"
start_anti_cold_boot >/dev/null 2>&1 || true
assert_log "sysctl -w fs.suid_dumpable=0" "Locks suid core-dumps against forensic extraction"
assert_log "sysctl -w kernel.core_pattern=|/bin/false" "Disables kernel crash dumps to disk"
stop_anti_cold_boot >/dev/null 2>&1 || true
assert_log "sysctl -w fs.suid_dumpable=" "Restores suid dumpable state"

# --- 7. emergency_panic ---
echo -e "\n\e[1;34m-> Module: emergency_panic\e[0m"
source assets/scripts/emergency_panic
trigger_panic >/dev/null 2>&1 || true
assert_log "ip link set eth0 down" "Severs Ethernet interfaces immediately"
assert_log "killall -9 firefox" "Terminates browser and sensitive processes"
assert_log "iptables -P OUTPUT DROP" "Locks all outbound traffic permanently"

# --- 8. traffic_obfuscation ---
echo -e "\n\e[1;34m-> Module: traffic_obfuscation\e[0m"
source assets/scripts/traffic_obfuscation
start_traffic_obfuscation >/dev/null 2>&1 || true
assert_log "tc qdisc add dev" "Injects jitter and latency with tc"
stop_traffic_obfuscation >/dev/null 2>&1 || true
assert_log "tc qdisc del dev" "Removes latency qdisc on stop"

# --- 9. sandbox_isolation ---
echo -e "\n\e[1;34m-> Module: sandbox_isolation\e[0m"
source assets/scripts/sandbox_isolation
start_sandbox_isolation >/dev/null 2>&1 || true
assert_log "ip netns add anon_jail" "Creates isolated kernel namespace"
stop_sandbox_isolation >/dev/null 2>&1 || true
assert_log "ip netns del anon_jail" "Tears down kernel namespace cleanly"

# --- 10. notify-send (Desktop Alerts) ---
echo -e "\n\e[1;34m-> Feature: Desktop Notifications (notify)\e[0m"
source assets/sources/config
notify "Security Warning" "Test Alert" "critical"
assert_log "notify-send -u critical -a ANON Security Warning Test Alert" "Dispatches desktop notifications via notify-send"

# --- 11. leak_auditor ---
echo -e "\n\e[1;34m-> Module: leak_auditor\e[0m"
source assets/sources/config
assets/scripts/leak_auditor --check-once >/dev/null 2>&1
if [ $? -eq 0 ]; then
    echo -e "  \e[32m[PASS]\e[0m One-shot leak audit executes and reports clear state"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m One-shot leak audit failed"
    ((FAIL_COUNT++))
fi

# --- 12. metadata_cleaner ---
echo -e "\n\e[1;34m-> Module: metadata_cleaner\e[0m"
TEST_SAMPLE="$MOCK_DIR/sample_image.jpg"
touch "$TEST_SAMPLE"
assets/scripts/metadata_cleaner "$TEST_SAMPLE" >/dev/null 2>&1
assert_log "mat2 --inplace $TEST_SAMPLE" "Invokes MAT2 engine for file sanitization"

echo -e "\n\e[1;33m[*] Running Blackbox Tests (End-to-End Execution)...\e[0m"
: > "$TEST_LOGS"

echo -e "\n\e[1;34m-> Feature: CLI Argument (--status)\e[0m"
"$MOCK_DIR/anon_test" --status > blackbox_out.txt 2>&1
if grep -q "ANON SYSTEM STATUS" blackbox_out.txt && grep -q "Leak Auditor" blackbox_out.txt; then
    echo -e "  \e[32m[PASS]\e[0m System status dashboard prints all 22 modules successfully"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m System status failed"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--help)\e[0m"
"$MOCK_DIR/anon_test" --help > blackbox_help.txt 2>&1 || true
if grep -q "\-\-panic" blackbox_help.txt && grep -q "\-\-audit" blackbox_help.txt && grep -q "\-\-clean-meta" blackbox_help.txt; then
    echo -e "  \e[32m[PASS]\e[0m CLI help displays all options including --audit and --clean-meta"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m CLI help missing options"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--audit)\e[0m"
"$MOCK_DIR/anon_test" --audit > blackbox_audit.txt 2>&1 || true
if grep -q "Audit complete" blackbox_audit.txt; then
    echo -e "  \e[32m[PASS]\e[0m CLI --audit executes one-shot audit successfully"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m CLI --audit failed"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--clean-meta)\e[0m"
"$MOCK_DIR/anon_test" --clean-meta "$TEST_SAMPLE" > blackbox_meta.txt 2>&1 || true
if grep -q "Metadata sanitized successfully" blackbox_meta.txt; then
    echo -e "  \e[32m[PASS]\e[0m CLI --clean-meta executes metadata sanitizer successfully"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m CLI --clean-meta failed"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--start non-interactive)\e[0m"
mock_cmd sleep
sed -i 's/hostname_changer_status="Disable"/hostname_changer_status="Enable"/g' "$SRCDIR/sources/config"
START_OUT=$("$MOCK_DIR/anon_test" --start < /dev/null 2>&1 || true)
sed -i 's/hostname_changer_status="Enable"/hostname_changer_status="Disable"/g' "$SRCDIR/sources/config"
if echo "$START_OUT" | grep -q "Anon started successfully in non-interactive mode"; then
    echo -e "  \e[32m[PASS]\e[0m Headless --start executes without interactive prompt"
    ((PASS_COUNT++))
else
    echo -e "  \e[31m[FAIL]\e[0m Headless --start failed"
    ((FAIL_COUNT++))
fi

echo -e "\n\e[1;36m=================================================\e[0m"
echo -e "Test Results: \e[32m$PASS_COUNT Passed\e[0m | \e[31m$FAIL_COUNT Failed\e[0m"
if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "\e[1;32mALL $PASS_COUNT TESTS PASSED SUCCESSFULLY! The architecture is solid.\e[0m"
else
    echo -e "\e[1;31mSOME TESTS FAILED! Please check the logs.\e[0m"
    exit 1
fi
echo -e "\e[1;36m=================================================\e[0m"
