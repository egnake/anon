use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::registry::ALL_MODULES;
use crate::sys::FileLock;

#[derive(Clone, Debug, Default)]
pub struct AnonState {
    entries: BTreeMap<String, String>,
}

impl AnonState {
    pub fn load(path: &Path) -> Self {
        let mut entries = BTreeMap::new();
        if let Ok(content) = fs::read_to_string(path) {
            for raw_line in content.lines() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    let key = k.trim();
                    if is_valid_ident(key) {
                        let val = v.trim().trim_matches('"').to_string();
                        entries.insert(key.to_string(), val);
                    }
                }
            }
        }
        Self { entries }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(|s| s.as_str())
    }

    pub fn is_enabled(&self, mod_name: &str) -> bool {
        let key = format!("{mod_name}_status");
        self.get(&key) == Some("Enable")
    }

    pub fn get_pid(&self, mod_name: &str) -> i32 {
        let key = format!("{mod_name}_pid");
        self.get(&key)
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0)
    }

    pub fn count_active(&self) -> usize {
        ALL_MODULES
            .iter()
            .filter(|m| self.is_enabled(m.name))
            .count()
    }
}

pub fn set_module_state(state_file: &Path, key: &str, val: &str) -> Result<(), String> {
    if !is_valid_ident(key) {
        return Err(format!("Invalid state key: {key}"));
    }
    let clean_val: String = val.chars().filter(|&c| c != '"' && c != '\n' && c != '\r').collect();

    if let Some(parent) = state_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
    }

    let lock_path = PathBuf::from(format!("{}.lock", state_file.display()));
    let _lock = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .ok()
        .and_then(FileLock::acquire);

    let mut state = AnonState::load(state_file);
    state.entries.insert(key.to_string(), clean_val);
    write_state_atomic(state_file, &state)
}

pub fn set_module_status(state_file: &Path, mod_name: &str, enabled: bool) -> Result<(), String> {
    let val = if enabled { "Enable" } else { "Disable" };
    set_module_state(state_file, &format!("{mod_name}_status"), val)
}

pub fn set_module_pid(state_file: &Path, mod_name: &str, pid: i32) -> Result<(), String> {
    set_module_state(state_file, &format!("{mod_name}_pid"), &pid.to_string())
}

pub fn reset_all_module_states(state_file: &Path) {
    let lock_path = PathBuf::from(format!("{}.lock", state_file.display()));
    let _ = fs::remove_file(state_file);
    let _ = fs::remove_file(lock_path);
}

fn write_state_atomic(state_file: &Path, state: &AnonState) -> Result<(), String> {
    let parent = state_file.parent().unwrap_or_else(|| Path::new("/tmp"));
    let pid = crate::sys::current_pid();
    let nonce = crate::helpers::rand_u64();
    let tmp_path = parent.join(format!(".anon_state_{pid}_{nonce:x}"));

    {
        let mut f = File::create(&tmp_path).map_err(|e| format!("create tmp state: {e}"))?;
        let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
        for (k, v) in &state.entries {
            writeln!(f, "{k}=\"{v}\"").map_err(|e| format!("write state: {e}"))?;
        }
        f.flush().map_err(|e| format!("flush state: {e}"))?;
    }

    fs::rename(&tmp_path, state_file).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("rename state: {e}")
    })
}

pub fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip_and_count() {
        let dir = std::env::temp_dir().join(format!("anon_state_test_{}", crate::helpers::rand_u64()));
        fs::create_dir_all(&dir).unwrap();
        let sf = dir.join("anon.state");

        set_module_status(&sf, "ip_changer", true).unwrap();
        set_module_status(&sf, "kill_switch", true).unwrap();
        set_module_pid(&sf, "anti_mitm", 4242).unwrap();

        let loaded = AnonState::load(&sf);
        assert_eq!(loaded.count_active(), 2);
        assert!(loaded.is_enabled("ip_changer"));
        assert!(!loaded.is_enabled("mac_changer"));
        assert_eq!(loaded.get_pid("anti_mitm"), 4242);

        reset_all_module_states(&sf);
        assert!(!sf.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}

