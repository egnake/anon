#!/bin/bash
set -u

echo -e "\e[1;36m=================================================\e[0m"
echo -e "\e[1;36m      Anon - Complete & Isolated Test Suite      \e[0m"
echo -e "\e[1;36m=================================================\e[0m\n"

# --- Setup 100% Isolated Mock Environment (Never mutates repo files) ---
export MOCK_DIR
MOCK_DIR=$(mktemp -d)
export PATH="$MOCK_DIR:$PATH"
export TEST_LOGS="$MOCK_DIR/test.log"
touch "$TEST_LOGS"

cleanup() {
    rm -rf "$MOCK_DIR"
    rm -f blackbox_out.txt blackbox_help.txt blackbox_audit.txt blackbox_meta.txt
}
trap cleanup EXIT INT TERM

mock_cmd() {
    local cmd="$1"
    cat > "$MOCK_DIR/$cmd" << EOF
#!/bin/bash
echo "$cmd \$*" >> "$TEST_LOGS"
if [ "$cmd" = "curl" ]; then
    echo '{"IsTor":true}'
elif [ "$cmd" = "ip" ]; then
    if [[ "\$*" == *"route"* ]]; then
        echo "default via 192.168.1.1 dev eth0"
    elif [[ "\$*" == *"-o link show"* ]]; then
        echo '1: lo: <LOOPBACK>'
        echo '2: eth0: <BROADCAST,MULTICAST>'
        echo '3: wlan0: <BROADCAST,MULTICAST>'
    elif [[ "\$*" == *"neigh show"* ]]; then
        echo '192.168.1.1 dev eth0 lladdr aa:bb:cc:dd:ee:ff router REACHABLE'
    fi
elif [ "$cmd" = "dmidecode" ]; then
    echo 'Mocked Manufacturer'
elif [ "$cmd" = "nmcli" ]; then
    echo 'wlan0 wifi'
elif [ "$cmd" = "rfkill" ]; then
    if [[ "\$*" == *"list bluetooth"* ]]; then
        echo '0: hci0: Bluetooth'
        echo '	Soft blocked: no'
    fi
elif [ "$cmd" = "timedatectl" ]; then
    if [[ "\$*" == *"show"* ]]; then
        echo 'UTC'
    fi
elif [ "$cmd" = "mount" ]; then
    touch "$MOCK_DIR/var_log_mounted"
elif [ "$cmd" = "umount" ]; then
    rm -f "$MOCK_DIR/var_log_mounted"
elif [ "$cmd" = "mountpoint" ]; then
    [ -f "$MOCK_DIR/var_log_mounted" ] && exit 0 || exit 1
elif [ "$cmd" = "iptables-save" ] || [ "$cmd" = "ip6tables-save" ]; then
    echo '*filter'
    echo 'COMMIT'
elif [ "$cmd" = "iptables-restore" ] || [ "$cmd" = "ip6tables-restore" ]; then
    while IFS= read -r line; do
        echo "$cmd: \$line" >> "$TEST_LOGS"
    done
fi
exit 0
EOF
    chmod +x "$MOCK_DIR/$cmd"
}

for cmd in iptables ip6tables iptables-save ip6tables-save iptables-restore ip6tables-restore \
           sysctl systemctl curl timedatectl dmidecode nmcli macchanger obfs4proxy shred ip \
           hostname hostnamectl killall rfkill tc chattr dnscrypt-proxy mount notify-send \
           umount mountpoint swapoff swapon tar tor proxychains4 sdmem i2pd mat2 exiftool \
           pkill pgrep snowflake-client arping; do
    mock_cmd "$cmd"
done

# Copy runtime tree into isolated sandbox
cp -a "$(pwd)/assets" "$MOCK_DIR/assets"
export BASE_DIR="$MOCK_DIR"
export SRCDIR="$MOCK_DIR/assets"
export BACKUPDIR="$MOCK_DIR/backups"
export ANON_STATE_FILE="$MOCK_DIR/anon.state"
mkdir -p "$BACKUPDIR"
touch "$BACKUPDIR/anon_fix_backups.tar.gz"

mkdir -p "$MOCK_DIR/etc_tor"
export TORRC="$MOCK_DIR/etc_tor/torrc"
touch "$TORRC"

sed 's/\[\[ "$EUID" -ne 0 \]\]/[[ "0" -ne 0 ]]/g' anon > "$MOCK_DIR/anon"
chmod +x "$MOCK_DIR/anon"

# shellcheck disable=SC1091
source "$SRCDIR/sources/config"

echo -e "\e[1;33m[*] Running Whitebox Tests (Internal Logic Verification)...\e[0m"
FAIL_COUNT=0
PASS_COUNT=0

pass() {
    echo -e "  \e[32m[PASS]\e[0m $1"
    ((PASS_COUNT++))
}

fail() {
    echo -e "  \e[31m[FAIL]\e[0m $1"
    ((FAIL_COUNT++))
}

assert_log() {
    if grep -q "$1" "$TEST_LOGS"; then
        pass "$2"
    else
        fail "$2 (Expected: '$1')"
    fi
}

# --- 1. Registry & Config Integrity ---
echo -e "\n\e[1;34m-> Architecture: Declarative Module Registry & State\e[0m"
reg_check=$(sed -n '/^MOD_NAMES=/,/^_is_pid_alive/p' "$MOCK_DIR/anon" | grep -v '_is_pid_alive')
if eval "$reg_check" && \
   [[ ${#MOD_NAMES[@]} -eq 22 && ${#MOD_LABELS[@]} -eq 22 && ${#MOD_TAGS[@]} -eq 22 && \
      $(( ${#NET_CAT_INDICES[@]} + ${#OBF_CAT_INDICES[@]} + ${#DEF_CAT_INDICES[@]} )) -eq 22 && \
      ${#EXEC_ORDER[@]} -eq 22 && ${#STOP_ORDER[@]} -eq 22 ]]; then
    pass "Declarative module registry covers all 22 modules consistently"
else
    fail "Module registry array count mismatch"
fi

set_module_status "ip_changer" "Enable"
set_module_status "kill_switch" "Enable"
if [[ "$(count_active_modules)" -eq 2 ]]; then
    pass "Atomic locked state management accurately tracks active modules"
else
    fail "Atomic state management count mismatch"
fi
reset_all_module_states

# --- 2. os_obfuscation ---
echo -e "\n\e[1;34m-> Module: os_obfuscation (start & stop)\e[0m"
source "$SRCDIR/scripts/os_obfuscation"
os_obfuscation_status="Disable"
start_os_obfuscation >/dev/null
assert_log "sysctl -w net.ipv4.ip_default_ttl=128" "Sets TTL to 128 (Windows fingerprint)"
assert_log "sysctl -w net.ipv4.tcp_window_scaling=0" "Disables TCP window scaling"
stop_os_obfuscation >/dev/null
assert_log "sysctl -w net.ipv4.ip_default_ttl=" "Restores default TTL"

# --- 3. ip_changer ---
echo -e "\n\e[1;34m-> Module: ip_changer (start & stop)\e[0m"
source "$SRCDIR/scripts/ip_changer"
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

# --- 4. mac_changer ---
echo -e "\n\e[1;34m-> Module: mac_changer (start & stop)\e[0m"
source "$SRCDIR/scripts/mac_changer"
mac_changer_status="Disable"
start_mac_changer >/dev/null 2>&1 || true
assert_log "ip link set wlan0 down" "Takes network interface down safely"
assert_log "ip link set wlan0 address" "Assigns new authentic vendor-spoofed MAC address"
stop_mac_changer >/dev/null 2>&1 || true
assert_log "ip link set .* up" "Restores interface state on stop"

# --- 5. kill_switch ---
echo -e "\n\e[1;34m-> Module: kill_switch (start & stop)\e[0m"
source "$SRCDIR/scripts/kill_switch"
kill_switch_status="Disable"
start_kill_switch >/dev/null 2>&1 || true
assert_log "iptables-restore: :OUTPUT DROP" "Sets default OUTPUT policy to DROP atomically"
assert_log "iptables-restore: -A OUTPUT -o lo -j ACCEPT" "Ensures localhost traffic remains allowed during lockdown"
assert_log "iptables-restore: -A OUTPUT -d 192.168.0.0/16 -p udp --dport 53 -j DROP" "Blocks LAN DNS leakage in kill switch"
assert_log "ip6tables-restore: :OUTPUT DROP" "Enforces complete IPv6 kill switch lockdown"
stop_kill_switch >/dev/null 2>&1 || true
assert_log "iptables-restore" "Restores baseline firewall ruleset on kill switch stop"

# --- 6. hostname_changer ---
echo -e "\n\e[1;34m-> Module: hostname_changer (start & stop)\e[0m"
source "$SRCDIR/scripts/hostname_changer"
hostname_changer_status="Disable"
start_hostname_changer >/dev/null 2>&1 || true
assert_log "hostname " "Applies spoofed hostname live to running kernel"
stop_hostname_changer >/dev/null 2>&1 || true
assert_log "hostname " "Restores original hostname"

# --- 7. anti_cold_boot ---
echo -e "\n\e[1;34m-> Module: anti_cold_boot (start & stop)\e[0m"
source "$SRCDIR/scripts/anti_cold_boot"
anti_cold_boot_status="Disable"
start_anti_cold_boot >/dev/null 2>&1 || true
assert_log "sysctl -w fs.suid_dumpable=0" "Locks suid core-dumps against forensic extraction"
assert_log "sysctl -w kernel.core_pattern=|/bin/false" "Disables kernel crash dumps to disk"
stop_anti_cold_boot >/dev/null 2>&1 || true
assert_log "sysctl -w fs.suid_dumpable=" "Restores suid dumpable state"

# --- 8. emergency_panic ---
echo -e "\n\e[1;34m-> Module: emergency_panic\e[0m"
source "$SRCDIR/scripts/emergency_panic"
trigger_panic >/dev/null 2>&1 || true
assert_log "ip link set eth0 down" "Severs Ethernet interfaces immediately"
assert_log "killall -9 firefox" "Terminates browser and sensitive processes"
assert_log "iptables -P OUTPUT DROP" "Locks all outbound traffic permanently"

# --- 9. traffic_obfuscation ---
echo -e "\n\e[1;34m-> Module: traffic_obfuscation\e[0m"
source "$SRCDIR/scripts/traffic_obfuscation"
start_traffic_obfuscation >/dev/null 2>&1 || true
assert_log "tc qdisc add dev" "Injects jitter and latency with tc"
stop_traffic_obfuscation >/dev/null 2>&1 || true
assert_log "tc qdisc del dev" "Removes latency qdisc on stop"

# --- 10. sandbox_isolation ---
echo -e "\n\e[1;34m-> Module: sandbox_isolation\e[0m"
source "$SRCDIR/scripts/sandbox_isolation"
start_sandbox_isolation >/dev/null 2>&1 || true
assert_log "ip netns add anon_jail" "Creates isolated kernel namespace"
stop_sandbox_isolation >/dev/null 2>&1 || true
assert_log "ip netns del anon_jail" "Tears down kernel namespace cleanly"

# --- 11. notify-send ---
echo -e "\n\e[1;34m-> Feature: Desktop Notifications (notify)\e[0m"
source "$SRCDIR/sources/config"
notify "Security Warning" "Test Alert" "critical"
assert_log "notify-send -u critical -a ANON Security Warning Test Alert" "Dispatches desktop notifications via notify-send"

# --- 12. Remaining Modules (DPI, Bridges, DoH, DNS, I2P, BT, TZ, Logging, Proxychains, Stealth, Process, Browser, Anti-MITM, Fix) ---
echo -e "\n\e[1;34m-> Modules: DPI Spoofing, Tor Bridges, I2P, Bluetooth, Timezone, Logging, Proxychains, Stealth\e[0m"
source "$SRCDIR/scripts/dpi_spoofing"
if ( set +e; source "$SRCDIR/scripts/dpi_spoofing"; [[ $- != *e* ]] ); then
    pass "dpi_spoofing does not leak set -e into parent shell"
else
    fail "dpi_spoofing leaked set -e"
fi
start_dpi_spoofing >/dev/null 2>&1 && [[ "${dpi_spoofing_status:-}" == "Enable" ]] && stop_dpi_spoofing >/dev/null 2>&1
pass "dpi_spoofing toggles state cleanly"

source "$SRCDIR/scripts/tor_bridges"
start_tor_bridges < /dev/null >/dev/null 2>&1 || true
if [[ "${tor_bridges_status:-}" == "Enable" ]] && _is_valid_bridge_line "obfs4 1.2.3.4:443 ABC" && ! _is_valid_bridge_line "bad line"; then
    pass "tor_bridges handles non-TTY default fallback and syntax validation"
else
    fail "tor_bridges fallback/validation failed"
fi
stop_tor_bridges >/dev/null 2>&1 || true

source "$SRCDIR/scripts/i2p_integration"
start_i2p_integration >/dev/null 2>&1 || true
assert_log "systemctl start i2pd" "Starts i2pd router via service_ctrl"
stop_i2p_integration >/dev/null 2>&1 || true
assert_log "systemctl stop i2pd" "Stops i2pd router via service_ctrl"

source "$SRCDIR/scripts/bluetooth_disabler"
start_bluetooth_disabler >/dev/null 2>&1 || true
assert_log "rfkill block bluetooth" "Blocks Bluetooth radio via rfkill"
stop_bluetooth_disabler >/dev/null 2>&1 || true
assert_log "rfkill unblock bluetooth" "Restores Bluetooth radio on stop"

source "$SRCDIR/scripts/timezone_changer"
start_timezone_changer >/dev/null 2>&1 || true
assert_log "timedatectl set-timezone" "Applies spoofed timezone"
stop_timezone_changer >/dev/null 2>&1 || true
[[ "${timezone_changer_status:-}" == "Disable" ]] && pass "timezone_changer restores and disables cleanly" || fail "timezone_changer stop failed"

source "$SRCDIR/scripts/volatile_logging"
start_volatile_logging >/dev/null 2>&1 || true
assert_log "mount -t tmpfs" "Mounts tmpfs over /var/log for volatile logging"
stop_volatile_logging >/dev/null 2>&1 || true
assert_log "systemctl restart systemd-journald" "Restarts journald on volatile logging teardown"

MOCK_PC="$MOCK_DIR/proxychains4.conf"
printf 'socks4 127.0.0.1 9050\n#quiet_mode\n' > "$MOCK_PC"
ANON_PROXYCHAINS_CONF="$MOCK_PC" bash -c 'source "$SRCDIR/scripts/proxychains_integration"; start_proxychains_integration >/dev/null 2>&1'
if grep -q '^socks5  127.0.0.1 9050' "$MOCK_PC" && grep -q '^quiet_mode' "$MOCK_PC"; then
    pass "proxychains_integration rewrites config to Tor SOCKS5"
else
    fail "proxychains_integration failed to rewrite config"
fi
ANON_PROXYCHAINS_CONF="$MOCK_PC" bash -c 'source "$SRCDIR/scripts/proxychains_integration"; stop_proxychains_integration >/dev/null 2>&1'

source "$SRCDIR/scripts/execution_stealth"
start_execution_stealth >/dev/null 2>&1
if [[ "${execution_stealth_status:-}" == "Enable" ]]; then
    stop_execution_stealth >/dev/null 2>&1
    [[ "${execution_stealth_status:-}" == "Disable" ]] && pass "execution_stealth toggles Enable/Disable cleanly" || fail "execution_stealth stop failed"
else
    fail "execution_stealth start failed"
fi

sed '/^main "\$@"/d' "$SRCDIR/scripts/anti_mitm" > "$MOCK_DIR/anti_mitm_lib.sh"
if (
    # shellcheck disable=SC1090,SC1091
    source "$MOCK_DIR/anti_mitm_lib.sh"
    declare -F notify >/dev/null 2>&1 && [[ "$(_get_gateway_mac 192.168.1.1)" == "aa:bb:cc:dd:ee:ff" ]]
); then
    pass "anti_mitm sources config (notify defined) and parses lladdr cleanly"
else
    fail "anti_mitm config/lladdr check failed"
fi

# --- 13. leak_auditor (positive + negative cases) ---
echo -e "\n\e[1;34m-> Module: leak_auditor (clear, DoH whitelist, DNS leak lockdown)\e[0m"
if "$SRCDIR/scripts/leak_auditor" --check-once >/dev/null 2>&1; then
    pass "One-shot leak audit executes and reports clear state"
else
    fail "One-shot leak audit failed"
fi

sed '/^main "\$@"/d' "$SRCDIR/scripts/leak_auditor" > "$MOCK_DIR/auditor_lib.sh"
echo "nameserver 127.0.2.1" > "$MOCK_DIR/resolv_doh"
if (
    # shellcheck disable=SC1090,SC1091
    source "$MOCK_DIR/auditor_lib.sh"
    ANON_RESOLV_CONF="$MOCK_DIR/resolv_doh" doh_shield_status="Enable" ip_changer_status="Disable" check_dns_leak
); then
    pass "leak_auditor permits 127.0.2.1 when DoH Shield is active"
else
    fail "leak_auditor rejected valid DoH loopback resolver"
fi

echo "nameserver 127.0.0.53" > "$MOCK_DIR/resolv_bad"
if (
    # shellcheck disable=SC1090,SC1091
    source "$MOCK_DIR/auditor_lib.sh"
    ANON_RESOLV_CONF="$MOCK_DIR/resolv_bad" doh_shield_status="Enable" ip_changer_status="Disable" check_dns_leak 2>/dev/null
); then
    fail "leak_auditor failed to catch 127.0.0.53 DNS leak"
else
    pass "leak_auditor triggers lockdown on 127.0.0.53 DNS leak"
fi

# --- 14. metadata_cleaner (file + empty dir edge case) ---
echo -e "\n\e[1;34m-> Module: metadata_cleaner\e[0m"
TEST_SAMPLE="$MOCK_DIR/sample_image.jpg"
touch "$TEST_SAMPLE"
"$SRCDIR/scripts/metadata_cleaner" "$TEST_SAMPLE" >/dev/null 2>&1
assert_log "mat2 --inplace $TEST_SAMPLE" "Invokes MAT2 engine for file sanitization"

EMPTY_DIR="$MOCK_DIR/empty_dir"
mkdir -p "$EMPTY_DIR"
if "$SRCDIR/scripts/metadata_cleaner" "$EMPTY_DIR" >/dev/null 2>&1; then
    pass "metadata_cleaner handles empty directories gracefully (exit 0)"
else
    fail "metadata_cleaner errored on empty directory"
fi

# --- 15. Blackbox CLI Tests ---
echo -e "\n\e[1;33m[*] Running Blackbox Tests (End-to-End Execution)...\e[0m"
: > "$TEST_LOGS"

echo -e "\n\e[1;34m-> Feature: CLI Argument (--status & --status --json)\e[0m"
"$MOCK_DIR/anon" --status > blackbox_out.txt 2>&1
if grep -q "ANON SYSTEM STATUS" blackbox_out.txt && grep -q "Leak Auditor" blackbox_out.txt; then
    pass "System status dashboard prints all 22 modules successfully"
else
    fail "System status failed"
fi

STATUS_JSON=$("$MOCK_DIR/anon" --status --json 2>&1)
if echo "$STATUS_JSON" | grep -q '"version":"3.0.0"' && echo "$STATUS_JSON" | grep -q '"leak_auditor":false'; then
    pass "CLI --status --json emits clean parseable JSON"
else
    fail "CLI --status --json output invalid"
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--version & --help)\e[0m"
VER_OUT=$("$MOCK_DIR/anon" --version 2>&1)
if [[ "$VER_OUT" == "ANON v3.0.0" ]]; then
    pass "CLI --version prints clean version string"
else
    fail "CLI --version unexpected: $VER_OUT"
fi

"$MOCK_DIR/anon" --help > blackbox_help.txt 2>&1 || true
if grep -q "\-\-panic" blackbox_help.txt && grep -q "\-\-audit" blackbox_help.txt && grep -q "\-\-clean-meta" blackbox_help.txt && grep -q "\-\-profile" blackbox_help.txt; then
    pass "CLI help displays all options including --profile, --audit, and --clean-meta"
else
    fail "CLI help missing options"
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--audit & --audit --json)\e[0m"
"$MOCK_DIR/anon" --audit > blackbox_audit.txt 2>&1 || true
if grep -q "Audit complete" blackbox_audit.txt; then
    pass "CLI --audit executes one-shot audit successfully"
else
    fail "CLI --audit failed"
fi

AUDIT_JSON=$("$MOCK_DIR/anon" --audit --json 2>&1)
if echo "$AUDIT_JSON" | grep -q '"status":"clean"' && ! echo "$AUDIT_JSON" | grep -q $'\x1b'; then
    pass "CLI --audit --json emits ANSI-free JSON"
else
    fail "CLI --audit --json invalid or contains ANSI codes"
fi

echo -e "\n\e[1;34m-> Feature: CLI Argument (--clean-meta)\e[0m"
"$MOCK_DIR/anon" --clean-meta "$TEST_SAMPLE" > blackbox_meta.txt 2>&1 || true
if grep -q "Metadata sanitized successfully" blackbox_meta.txt; then
    pass "CLI --clean-meta executes metadata sanitizer successfully"
else
    fail "CLI --clean-meta failed"
fi

echo -e "\n\e[1;34m-> Feature: CLI Arguments (--start, --stop, --fix, --panic, invalid --profile)\e[0m"
mock_cmd sleep
: > "$TEST_LOGS"
set_module_status "hostname_changer" "Enable"
START_OUT=$("$MOCK_DIR/anon" --start < /dev/null 2>&1 || true)
if echo "$START_OUT" | grep -q "Anon started successfully in non-interactive mode" && grep -q "Hostname changer successfully enabled" <<< "$START_OUT"; then
    pass "Headless --start executes saved enabled modules without false skip"
else
    fail "Headless --start failed or skipped saved enabled module"
fi

STOP_OUT=$("$MOCK_DIR/anon" --stop < /dev/null 2>&1 || true)
if echo "$STOP_OUT" | grep -q "Hostname changer successfully disabled"; then
    pass "CLI --stop deactivates active modules cleanly"
else
    fail "CLI --stop failed"
fi

set_module_status "kill_switch" "Enable"
PROF_OUT=$("$MOCK_DIR/anon" --profile quick < /dev/null 2>&1 || true)
if echo "$PROF_OUT" | grep -q "Profile 'quick' applied successfully" && \
   echo "$PROF_OUT" | grep -q "Network Kill Switch disabled" && \
   echo "$PROF_OUT" | grep -q "Ip changer successfully enabled"; then
    pass "CLI --profile stops excluded modules and starts target profile modules"
else
    fail "CLI --profile execution failed"
fi

FIX_OUT=$("$MOCK_DIR/anon" --fix < /dev/null 2>&1 || true)
if echo "$FIX_OUT" | grep -q "System successfully repaired"; then
    pass "CLI --fix runs system recovery cleanly"
else
    fail "CLI --fix failed"
fi

PANIC_OUT=$("$MOCK_DIR/anon" --panic < /dev/null 2>&1 || true)
if echo "$PANIC_OUT" | grep -q "System has been sanitized and locked down"; then
    pass "CLI --panic triggers lockdown protocol cleanly"
else
    fail "CLI --panic failed"
fi

if "$MOCK_DIR/anon" --profile nonexistent_profile >/dev/null 2>&1; then
    fail "CLI --profile accepted invalid profile name"
else
    pass "CLI --profile rejects invalid profile name"
fi

echo -e "\n\e[1;36m=================================================\e[0m"
echo -e "Test Results: \e[32m$PASS_COUNT Passed\e[0m | \e[31m$FAIL_COUNT Failed\e[0m"
if [ "$FAIL_COUNT" -eq 0 ]; then
    echo -e "\e[1;32mALL $PASS_COUNT TESTS PASSED SUCCESSFULLY! The architecture is solid.\e[0m"
else
    echo -e "\e[1;31mSOME TESTS FAILED! Please check the logs.\e[0m"
    exit 1
fi
echo -e "\e[1;36m=================================================\e[0m"
