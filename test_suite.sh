#!/bin/bash
# ==============================================================================
# ANON Framework — Automated Rust + Blackbox CLI Test Suite (Isolated Mock Env)
# ==============================================================================
set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS_COUNT=0
FAIL_COUNT=0

GREEN='\033[1;32m'
RED='\033[1;31m'
CYAN='\033[1;36m'
YELLOW='\033[1;33m'
RESET='\033[0m'

pass() {
    echo -e "  ${GREEN}[PASS]${RESET} $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo -e "  ${RED}[FAIL]${RESET} $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

assert_eq() {
    local actual="$1"
    local expected="$2"
    local label="$3"
    if [[ "$actual" == "$expected" ]]; then
        pass "$label"
    else
        fail "$label (expected: '$expected', got: '$actual')"
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local label="$3"
    if [[ "$haystack" == *"$needle"* ]]; then
        pass "$label"
    else
        fail "$label (missing: '$needle')"
    fi
}

assert_not_contains() {
    local haystack="$1"
    local needle="$2"
    local label="$3"
    if [[ "$haystack" != *"$needle"* ]]; then
        pass "$label"
    else
        fail "$label (unexpectedly contained: '$needle')"
    fi
}

echo -e "${CYAN}=== 1. Cargo Unit Tests & Release Build ===${RESET}"
if command -v cargo >/dev/null 2>&1; then
    if RUSTFLAGS="-D warnings" cargo test --manifest-path "$SCRIPT_DIR/Cargo.toml" --quiet; then
        pass "Cargo unit tests pass with -D warnings"
    else
        fail "Cargo unit tests failed"
    fi

    if [[ ! -x "$SCRIPT_DIR/target/release/anon" ]] || [[ "${ANON_FORCE_BUILD:-0}" == "1" ]]; then
        if RUSTFLAGS="-D warnings" cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml" --quiet; then
            pass "Cargo release binary compiled cleanly"
        else
            fail "Cargo release build failed"
        fi
    else
        pass "Using compiled release binary target/release/anon"
    fi
else
    fail "cargo not found in PATH"
fi

BIN_PATH="$SCRIPT_DIR/target/release/anon"
if [[ ! -x "$BIN_PATH" ]]; then
    echo -e "${RED}[FATAL] Binary missing at $BIN_PATH${RESET}"
    exit 1
fi

if ! command -v fakeroot >/dev/null 2>&1; then
    echo -e "${RED}[FATAL] fakeroot is required to run isolated CLI tests${RESET}"
    exit 1
fi

# ------------------------------------------------------------------------------
# Create Isolated Mock Environment
# ------------------------------------------------------------------------------
MOCK_DIR="$(mktemp -d /tmp/t_anon_XXXXXX)"
MOCK_BIN="${MOCK_DIR}/bin"
MOCK_LOG="${MOCK_DIR}/mock_calls.log"
trap 'rm -rf "$MOCK_DIR"' EXIT

mkdir -p "$MOCK_BIN" "${MOCK_DIR}/assets/sources" "${MOCK_DIR}/assets/banners" \
         "${MOCK_DIR}/backups" "${MOCK_DIR}/etc/tor" "${MOCK_DIR}/netns_etc" "${MOCK_DIR}/wrappers"

cp "$BIN_PATH" "${MOCK_DIR}/anon"
chmod +x "${MOCK_DIR}/anon"
cp -a "${SCRIPT_DIR}/assets/sources/." "${MOCK_DIR}/assets/sources/"
cp -a "${SCRIPT_DIR}/assets/banners/." "${MOCK_DIR}/assets/banners/" 2>/dev/null || true

touch "$MOCK_LOG"
printf 'TransPort 9040\nDNSPort 5353\n' > "${MOCK_DIR}/etc/tor/torrc"
printf '127.0.0.1 localhost\n' > "${MOCK_DIR}/etc/hosts"
printf 'nameserver 1.1.1.1\n' > "${MOCK_DIR}/etc/resolv.conf"
printf 'socks4 127.0.0.1 9050\n' > "${MOCK_DIR}/etc/proxychains4.conf"

create_mock_bin() {
    local name="$1"
    local body="$2"
    cat > "${MOCK_BIN}/${name}" <<EOF
#!/bin/bash
echo "${name} \$*" >> "$MOCK_LOG"
${body}
EOF
    chmod +x "${MOCK_BIN}/${name}"
}

create_mock_bin "iptables" "exit 0"
create_mock_bin "ip6tables" "exit 0"
create_mock_bin "iptables-save" "echo '*filter'; echo ':INPUT ACCEPT [0:0]'; echo ':FORWARD ACCEPT [0:0]'; echo ':OUTPUT ACCEPT [0:0]'; echo 'COMMIT'; exit 0"
create_mock_bin "ip6tables-save" "echo '*filter'; echo ':INPUT ACCEPT [0:0]'; echo ':FORWARD ACCEPT [0:0]'; echo ':OUTPUT ACCEPT [0:0]'; echo 'COMMIT'; exit 0"
create_mock_bin "iptables-restore" "cat >/dev/null; exit 0"
create_mock_bin "ip6tables-restore" "cat >/dev/null; exit 0"
create_mock_bin "systemctl" "if [[ \"\$1\" == \"is-active\" ]]; then exit 0; fi; exit 0"
create_mock_bin "service" "exit 0"
create_mock_bin "ip" 'case "$*" in *route*) echo "default via 192.168.1.1 dev eth0" ;; *neigh*) echo "192.168.1.1 dev eth0 lladdr 00:11:22:33:44:55 REACHABLE" ;; *link*) echo "link/ether 00:11:22:33:44:55 brd ff:ff:ff:ff:ff:ff" ;; esac; exit 0'
create_mock_bin "tc" "exit 0"
create_mock_bin "arping" "exit 0"
create_mock_bin "ping" "exit 0"
create_mock_bin "firefox" "exit 0"
create_mock_bin "timedatectl" "if [[ \"\$1\" == \"show\" ]]; then echo 'Europe/Istanbul'; fi; exit 0"
create_mock_bin "hostnamectl" "exit 0"
create_mock_bin "hostname" "if [[ \$# -eq 0 ]]; then echo 'testhost'; fi; exit 0"
create_mock_bin "macchanger" "exit 0"
create_mock_bin "proxychains4" "exit 0"
create_mock_bin "sdmem" "exit 0"
create_mock_bin "sswap" "exit 0"
create_mock_bin "nmcli" "exit 0"
create_mock_bin "xhost" "exit 0"
create_mock_bin "apparmor_parser" "exit 0"
create_mock_bin "firefox-esr" "exit 0"
create_mock_bin "rfkill" "echo '0: hci0: Bluetooth'; echo 'Soft blocked: no'; exit 0"
create_mock_bin "swapoff" "exit 0"
create_mock_bin "swapon" "exit 0"
create_mock_bin "tor" "exit 0"
create_mock_bin "i2pd" "exit 0"
create_mock_bin "dnscrypt-proxy" "exit 0"
create_mock_bin "firejail" "exit 0"
create_mock_bin "aa-enforce" "exit 0"
create_mock_bin "aa-disable" "exit 0"
create_mock_bin "mat2" "for arg in \"\$@\"; do if [[ -f \"\$arg\" ]]; then : > \"\${arg}.cleaned\"; fi; done; exit 0"
create_mock_bin "exiftool" "exit 0"
create_mock_bin "curl" "echo '{\"IsTor\":true,\"IP\":\"198.51.100.2\"}'; exit 0"
create_mock_bin "pkill" "exit 0"
create_mock_bin "killall" "exit 0"
create_mock_bin "pgrep" "exit 1"
create_mock_bin "ss" "exit 0"
create_mock_bin "mount" "exit 0"
create_mock_bin "umount" "exit 0"
create_mock_bin "mountpoint" "exit 1"
create_mock_bin "sysctl" "exit 0"
create_mock_bin "chattr" "exit 0"
create_mock_bin "shred" "exit 0"
create_mock_bin "notify-send" "exit 0"
create_mock_bin "obfs4proxy" "exit 0"
create_mock_bin "snowflake-client" "exit 0"
create_mock_bin "dmidecode" "echo 'MockVendor'; exit 0"

export PATH="${MOCK_BIN}:$PATH"
export BASE_DIR="$MOCK_DIR"
export BACKUPDIR="${MOCK_DIR}/backups"
export ANON_STATE_FILE="${MOCK_DIR}/backups/anon.state"
export TORRC="${MOCK_DIR}/etc/tor/torrc"
export ANON_HOSTS_FILE="${MOCK_DIR}/etc/hosts"
export ANON_RESOLV_CONF="${MOCK_DIR}/etc/resolv.conf"
export ANON_PROXYCHAINS_CONF="${MOCK_DIR}/etc/proxychains4.conf"
export APPARMOR_TOR_PROFILE="${MOCK_DIR}/apparmor.tor"
export NETNS_ETC_DIR="${MOCK_DIR}/netns_etc"
export WRAPPER_DIR="${MOCK_DIR}/wrappers"
unset SUDO_USER
export ANON_SKIP_STEALTH=1
export ANON_TOR_BOOTSTRAP_TIMEOUT=1

run_anon_root() {
    fakeroot "${MOCK_DIR}/anon" "$@"
}

# ------------------------------------------------------------------------------
# 2. CLI Basic, Root Guard, Version, Help, Status & JSON Schema
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 2. CLI Surface, Help, Root Guard & JSON Schema ===${RESET}"

VER_OUT="$("${MOCK_DIR}/anon" --version 2>&1)"
assert_contains "$VER_OUT" "3.0.0" "anon --version reports 3.0.0"

HELP_OUT="$("${MOCK_DIR}/anon" --help 2>&1)"
assert_contains "$HELP_OUT" "--start" "anon --help documents --start"
assert_contains "$HELP_OUT" "--audit" "anon --help documents --audit"
assert_contains "$HELP_OUT" "--clean-meta" "anon --help documents --clean-meta"

if [[ "$(id -u)" -ne 0 ]]; then
    "${MOCK_DIR}/anon" --status >/dev/null 2>&1
    NONROOT_RC=$?
    if [[ "$NONROOT_RC" -ne 0 ]]; then
        pass "Non-root execution of --status is rejected (exit $NONROOT_RC)"
    else
        fail "Non-root execution unexpectedly succeeded"
    fi
else
    pass "Running natively as UID 0; skipping non-root rejection test"
fi

STATUS_OUT="$(run_anon_root --status 2>&1)"
assert_contains "$STATUS_OUT" "Ip changer" "anon --status renders module labels"
assert_contains "$STATUS_OUT" "ANON SYSTEM STATUS" "anon --status renders header"

JSON_OUT="$(run_anon_root --status --json 2>&1)"
assert_contains "$JSON_OUT" '"version":"3.0.0"' "--status --json includes version"
assert_contains "$JSON_OUT" '"active_count":0' "--status --json shows 0 active modules initially"
assert_contains "$JSON_OUT" '"anti_cold_boot":false' "--status --json includes 22nd module anti_cold_boot"

# ------------------------------------------------------------------------------
# 3. Leak Auditor (--audit & --audit --json Exit Code Propagation)
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 3. Leak Auditor & Exit Code Propagation ===${RESET}"

AUDIT_OK_OUT="$(run_anon_root --audit 2>&1)"
AUDIT_OK_RC=$?
assert_eq "$AUDIT_OK_RC" "0" "--audit returns exit 0 when clean"
assert_contains "$AUDIT_OK_OUT" "No leaks detected" "--audit reports clean state"

# Simulate Tor active with clearnet nameserver 1.1.1.1 -> MUST exit 2 in BOTH modes!
printf 'ip_changer_status="Enable"\n' > "$ANON_STATE_FILE"
printf 'nameserver 1.1.1.1\n' > "$ANON_RESOLV_CONF"

AUDIT_LEAK_OUT="$(run_anon_root --audit 2>&1)"
AUDIT_LEAK_RC=$?
assert_eq "$AUDIT_LEAK_RC" "2" "--audit returns exit 2 on clearnet DNS leak"
assert_contains "$AUDIT_LEAK_OUT" "CRITICAL LEAK DETECTED" "--audit prints leak warning"

AUDIT_LEAK_JSON="$(run_anon_root --audit --json 2>&1)"
AUDIT_LEAK_JSON_RC=$?
assert_eq "$AUDIT_LEAK_JSON_RC" "2" "--audit --json returns exit 2 on DNS leak"
assert_contains "$AUDIT_LEAK_JSON" '"status":"leak_detected"' "--audit --json reports leak_detected"

: > "$ANON_STATE_FILE"

# ------------------------------------------------------------------------------
# 4. Metadata Cleaner (Parallel Dir + Empty + Missing Path)
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 4. Metadata Cleaner ===${RESET}"

META_DIR="${MOCK_DIR}/meta_samples"
mkdir -p "${META_DIR}/nested"
printf 'img1' > "${META_DIR}/a.jpg"
printf 'img2' > "${META_DIR}/nested/b.png"

META_OUT="$(run_anon_root --clean-meta "$META_DIR" 2>&1)"
META_RC=$?
assert_eq "$META_RC" "0" "--clean-meta succeeds on directory with files"
assert_contains "$META_OUT" "(2/2 file(s))" "--clean-meta sanitized both files in parallel"

run_anon_root --clean-meta "${MOCK_DIR}/does_not_exist" >/dev/null 2>&1
MISSING_RC=$?
if [[ "$MISSING_RC" -ne 0 ]]; then
    pass "--clean-meta fails on nonexistent path"
else
    fail "--clean-meta unexpectedly succeeded on nonexistent path"
fi

# ------------------------------------------------------------------------------
# 5. Profile Validation, Module Execution & Orphaned torrc.bak Sandbox Fix
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 5. Profiles, Start/Stop & torrc.bak Sandbox Recovery ===${RESET}"

BAD_PROF_OUT="$(run_anon_root --profile nonexistent_profile 2>&1)"
BAD_PROF_RC=$?
if [[ "$BAD_PROF_RC" -ne 0 ]]; then
    pass "--profile rejects unknown profile name"
else
    fail "--profile accepted unknown profile"
fi
assert_contains "$BAD_PROF_OUT" "Unknown profile" "--profile prints descriptive error"

# Orphaned torrc.bak recovery should strip '# ANON_SANDBOX' when sandbox_isolation is Disable
printf 'SocksPort 9050\nSandbox 1 # ANON_SANDBOX\nTransPort 9040\n' > "${MOCK_DIR}/backups/torrc.bak"
STOP_ORPHAN_OUT="$(run_anon_root --stop 2>&1)"
TORRC_AFTER="$(cat "$TORRC")"
assert_not_contains "$TORRC_AFTER" "# ANON_SANDBOX" "Orphaned torrc.bak restore strips stale # ANON_SANDBOX"
assert_contains "$STOP_ORPHAN_OUT" "Ip changer successfully disabled" "--stop tears down orphaned ip_changer cleanly"

# Test profile quick (enables ip_changer [2], browser_anonymization [7], sandbox_isolation [17])
PROF_OUT="$(run_anon_root --profile quick 2>&1)"
assert_contains "$PROF_OUT" "Profile 'quick' applied successfully" "--profile quick applies cleanly"
STATE_AFTER_QUICK="$(cat "$ANON_STATE_FILE" 2>/dev/null || true)"
assert_contains "$STATE_AFTER_QUICK" 'ip_changer_status="Enable"' "quick profile enables ip_changer"
assert_contains "$STATE_AFTER_QUICK" 'sandbox_isolation_status="Enable"' "quick profile enables sandbox_isolation"
TORRC_SANDBOX="$(cat "$TORRC" 2>/dev/null || true)"
assert_contains "$TORRC_SANDBOX" "# ANON_SANDBOX" "sandbox_isolation + ip_changer injects ANON_SANDBOX Tor listeners"
TORRC_BAK_QUICK="$(cat "${MOCK_DIR}/backups/torrc.bak" 2>/dev/null || true)"
assert_not_contains "$TORRC_BAK_QUICK" "# ANON_SANDBOX" "torrc.bak never stores # ANON_SANDBOX markers"

run_anon_root --stop >/dev/null 2>&1
JSON_AFTER_STOP="$(run_anon_root --status --json 2>&1)"
assert_contains "$JSON_AFTER_STOP" '"active_count":0' "--stop resets active_count to 0"
TORRC_AFTER_STOP="$(cat "$TORRC" 2>/dev/null || true)"
assert_not_contains "$TORRC_AFTER_STOP" "# ANON_SANDBOX" "Stopping sandbox+ip_changer leaves torrc free of # ANON_SANDBOX"

# Test profile lockdown (anti_mitm [0], volatile_logging [1], bluetooth_disabler [9], anti_cold_boot [12], kill_switch [14], execution_stealth [20])
LOCK_OUT="$(run_anon_root --profile lockdown 2>&1)"
assert_contains "$LOCK_OUT" "Profile 'lockdown' applied successfully" "--profile lockdown applies cleanly"
STATE_AFTER_LOCK="$(cat "$ANON_STATE_FILE" 2>/dev/null || true)"
assert_contains "$STATE_AFTER_LOCK" 'anti_cold_boot_status="Enable"' "lockdown profile enables anti_cold_boot"
assert_contains "$STATE_AFTER_LOCK" 'kill_switch_status="Enable"' "lockdown profile enables kill_switch"

run_anon_root --stop >/dev/null 2>&1

# Test default --start (activates ip_changer + kill_switch)
START_OUT="$(run_anon_root --start 2>&1)"
assert_contains "$START_OUT" "Anon started successfully" "--start completes cleanly"
STATE_AFTER_START="$(cat "$ANON_STATE_FILE" 2>/dev/null || true)"
assert_contains "$STATE_AFTER_START" 'ip_changer_status="Enable"' "--start enables ip_changer by default"
assert_contains "$STATE_AFTER_START" 'kill_switch_status="Enable"' "--start enables kill_switch by default"
run_anon_root --stop >/dev/null 2>&1

# ------------------------------------------------------------------------------
# 6. --fix & --panic Safety Flows
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 6. Emergency Fix & Panic Flows ===${RESET}"

# --profile quick already generated anon_fix_backups.tar.gz in backups/
FIX_OUT="$(run_anon_root --fix 2>&1)"
assert_contains "$FIX_OUT" "System successfully repaired" "--fix restores system backup cleanly"

printf 'anti_cold_boot_status="Enable"\n' > "$ANON_STATE_FILE"
PANIC_OUT="$(printf 'DELETE\n' | run_anon_root --panic 2>&1)"
assert_contains "$PANIC_OUT" "SUCCESS: System has been sanitized and locked down" "--panic executes emergency lockdown on DELETE"
JSON_AFTER_PANIC="$(run_anon_root --status --json 2>&1)"
assert_contains "$JSON_AFTER_PANIC" '"active_count":0' "--panic clears module state"

# ------------------------------------------------------------------------------
# 7. Exhaustive Subsystem, Daemon, Stealth, Browser & Edge-Case Coverage
# ------------------------------------------------------------------------------
echo -e "${CYAN}=== 7. Daemons, Remaining Profiles, Stealth, Browser & Edge Cases ===${RESET}"

# Invalid CLI flag should exit non-zero
INVALID_OUT="$(run_anon_root --not-a-real-flag 2>&1)"
INVALID_RC=$?
if [[ "$INVALID_RC" -ne 0 ]]; then
    pass "Invalid CLI flag exits non-zero ($INVALID_RC)"
else
    fail "Invalid CLI flag unexpectedly exited 0"
fi
assert_contains "$INVALID_OUT" "Invalid option" "Invalid CLI flag prints error hint"

# DoH Shield audit checks (127.0.2.1 allowed, 8.8.8.8 caught as leak)
printf 'doh_shield_status="Enable"\n' > "$ANON_STATE_FILE"
printf 'nameserver 127.0.2.1\n' > "$ANON_RESOLV_CONF"
DOH_CLEAN_RC=0
run_anon_root --audit >/dev/null 2>&1 || DOH_CLEAN_RC=$?
assert_eq "$DOH_CLEAN_RC" "0" "--audit allows 127.0.2.1 when doh_shield is enabled"

printf 'nameserver 8.8.8.8\n' > "$ANON_RESOLV_CONF"
DOH_LEAK_RC=0
run_anon_root --audit >/dev/null 2>&1 || DOH_LEAK_RC=$?
assert_eq "$DOH_LEAK_RC" "2" "--audit detects clearnet 8.8.8.8 leak when doh_shield is enabled"
: > "$ANON_STATE_FILE"
printf 'nameserver 1.1.1.1\n' > "$ANON_RESOLV_CONF"

# Metadata cleaner with spaces in filenames and directory names
SPACE_DIR="${MOCK_DIR}/dir with spaces"
mkdir -p "$SPACE_DIR"
printf 'meta1' > "${SPACE_DIR}/photo with spaces.jpg"
printf 'meta2' > "${SPACE_DIR}/doc 2.png"
SPACE_META_OUT="$(run_anon_root --clean-meta "$SPACE_DIR" 2>&1)"
assert_contains "$SPACE_META_OUT" "(2/2 file(s))" "--clean-meta handles paths/filenames with spaces"

# Remaining profiles: darknet, pentest, paranoia, ghost (includes live daemons)
DARKNET_OUT="$(run_anon_root --profile darknet 2>&1)"
assert_contains "$DARKNET_OUT" "Profile 'darknet' applied successfully" "--profile darknet applies cleanly"
assert_contains "$(cat "$ANON_STATE_FILE")" 'tor_bridges_status="Enable"' "darknet profile enables tor_bridges"
assert_contains "$(cat "$ANON_STATE_FILE")" 'i2p_integration_status="Enable"' "darknet profile enables i2p_integration"
run_anon_root --stop >/dev/null 2>&1

PENTEST_OUT="$(run_anon_root --profile pentest 2>&1)"
assert_contains "$PENTEST_OUT" "Profile 'pentest' applied successfully" "--profile pentest applies cleanly"
PENTEST_STATE="$(cat "$ANON_STATE_FILE")"
assert_contains "$PENTEST_STATE" 'proxychains_integration_status="Enable"' "pentest profile enables proxychains_integration"
assert_contains "$PENTEST_STATE" 'traffic_obfuscation_status="Enable"' "pentest profile enables traffic_obfuscation"
assert_contains "$PENTEST_STATE" 'anti_mitm_status="Enable"' "pentest profile enables live anti_mitm daemon"

MITM_PID="$(sed -n 's/^anti_mitm_pid="\([0-9]*\)"/\1/p' "$ANON_STATE_FILE")"
TRAFF_PID="$(sed -n 's/^traffic_obfuscation_pid="\([0-9]*\)"/\1/p' "$ANON_STATE_FILE")"
if [[ -n "$MITM_PID" ]] && [[ "$MITM_PID" -gt 0 ]] && kill -0 "$MITM_PID" 2>/dev/null; then
    pass "anti_mitm background daemon is genuinely alive (PID $MITM_PID)"
else
    fail "anti_mitm daemon failed to stay alive (PID '${MITM_PID:-none}')"
fi
if [[ -n "$TRAFF_PID" ]] && [[ "$TRAFF_PID" -gt 0 ]] && kill -0 "$TRAFF_PID" 2>/dev/null; then
    pass "traffic_obfuscation chaffing daemon is genuinely alive (PID $TRAFF_PID)"
else
    fail "traffic_obfuscation daemon failed to stay alive (PID '${TRAFF_PID:-none}')"
fi

run_anon_root --stop >/dev/null 2>&1
sleep 0.15
if [[ -n "$MITM_PID" ]] && ! kill -0 "$MITM_PID" 2>/dev/null; then
    pass "anti_mitm daemon terminated cleanly on --stop"
else
    fail "anti_mitm daemon remained running after --stop"
fi
if [[ -n "$TRAFF_PID" ]] && ! kill -0 "$TRAFF_PID" 2>/dev/null; then
    pass "traffic_obfuscation daemon terminated cleanly on --stop"
else
    fail "traffic_obfuscation daemon remained running after --stop"
fi

GHOST_OUT="$(run_anon_root --profile ghost 2>&1)"
assert_contains "$GHOST_OUT" "Profile 'ghost' applied successfully" "--profile ghost applies all 19 ghost modules cleanly"
AUDIT_PID="$(sed -n 's/^leak_auditor_pid="\([0-9]*\)"/\1/p' "$ANON_STATE_FILE")"
if [[ -n "$AUDIT_PID" ]] && [[ "$AUDIT_PID" -gt 0 ]] && kill -0 "$AUDIT_PID" 2>/dev/null; then
    pass "leak_auditor background daemon is genuinely alive (PID $AUDIT_PID)"
else
    fail "leak_auditor daemon failed to stay alive (PID '${AUDIT_PID:-none}')"
fi

# Verify execution_stealth live RAM re-exec (without ANON_SKIP_STEALTH) + clean Exit cleanup
STEALTH_JSON="$(env -u ANON_SKIP_STEALTH fakeroot "${MOCK_DIR}/anon" --status --json 2>&1)"
assert_contains "$STEALTH_JSON" '"execution_stealth":true' "execution_stealth re-exec preserves valid --status --json output"
sleep 0.1
LEFT_KWORKER=0
for d in /dev/shm/.kworker_*; do
    if [[ -e "$d" ]]; then
        LEFT_KWORKER=$((LEFT_KWORKER + 1))
    fi
done
assert_eq "$LEFT_KWORKER" "0" "execution_stealth StealthGuard removes temporary /dev/shm/.kworker_* on exit"

run_anon_root --stop >/dev/null 2>&1

# Disposable browser launch with mocked firefox
BROWSER_OUT="$(run_anon_root --browser 2>&1)"
assert_contains "$BROWSER_OUT" "Disposable profile completely wiped" "--browser launches and securely wipes disposable RAM profile"

# ------------------------------------------------------------------------------
# 8. Internationalization (--lang en|tr & persistence across --stop)
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}=== 8. Language Switcher (English / Türkçe) ===${RESET}"
HELP_LANG="$("${MOCK_DIR}/anon" --help 2>&1 || true)"
assert_contains "$HELP_LANG" "--lang" "anon --help documents --lang"

LANG_TR_OUT="$(run_anon_root --lang tr 2>&1)"
assert_contains "$LANG_TR_OUT" "Türkçe" "--lang tr switches active UI language to Turkish"

TR_STATUS="$(run_anon_root --status 2>&1)"
assert_contains "$TR_STATUS" "IP Değiştirici" "--status renders localized Turkish module labels"

run_anon_root --stop >/dev/null 2>&1
TR_STATUS_AFTER_STOP="$(run_anon_root --status 2>&1)"
assert_contains "$TR_STATUS_AFTER_STOP" "IP Değiştirici" "Language preference persists in anon.lang across --stop state resets"

LANG_EN_OUT="$(run_anon_root --lang en 2>&1)"
assert_contains "$LANG_EN_OUT" "English" "--lang en restores active UI language to English"
EN_STATUS="$(run_anon_root --status 2>&1)"
assert_contains "$EN_STATUS" "Ip changer" "--status renders default English module labels after --lang en"

# ------------------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------------------
echo ""
echo -e "${YELLOW}============================================================${RESET}"
echo -e "  Test Results: ${GREEN}${PASS_COUNT} Passed${RESET} | ${RED}${FAIL_COUNT} Failed${RESET}"
echo -e "${YELLOW}============================================================${RESET}"

if [[ "$FAIL_COUNT" -ne 0 ]]; then
    exit 1
fi
exit 0
