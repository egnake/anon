use crate::config::AppConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModuleMeta {
    pub index: usize,
    pub name: &'static str,
    pub label: &'static str,
    pub tag: &'static str,
}

pub const ALL_MODULES: [ModuleMeta; 22] = [
    ModuleMeta { index: 0,  name: "anti_mitm",               label: "Anti Mitm",               tag: "MITM " },
    ModuleMeta { index: 1,  name: "volatile_logging",        label: "Volatile logging",        tag: "VLOG " },
    ModuleMeta { index: 2,  name: "ip_changer",              label: "Ip changer",              tag: "IP   " },
    ModuleMeta { index: 3,  name: "dns_changer",             label: "Dns changer",             tag: "DNS  " },
    ModuleMeta { index: 4,  name: "mac_changer",             label: "Mac changer",             tag: "MAC  " },
    ModuleMeta { index: 5,  name: "timezone_changer",        label: "Timezone changer",        tag: "TZ   " },
    ModuleMeta { index: 6,  name: "hostname_changer",        label: "Hostname changer",        tag: "HOST " },
    ModuleMeta { index: 7,  name: "browser_anonymization",   label: "Browser anonymization",   tag: "BRWS " },
    ModuleMeta { index: 8,  name: "os_obfuscation",          label: "OS Obfuscation",          tag: "OS   " },
    ModuleMeta { index: 9,  name: "bluetooth_disabler",      label: "Bluetooth disabler",      tag: "BT   " },
    ModuleMeta { index: 10, name: "process_obfuscation",     label: "Process obfuscation",     tag: "PROC " },
    ModuleMeta { index: 11, name: "proxychains_integration", label: "Proxychains integration", tag: "PRXY " },
    ModuleMeta { index: 12, name: "anti_cold_boot",          label: "Anti cold boot",          tag: "COLD " },
    ModuleMeta { index: 13, name: "tor_bridges",             label: "Tor bridges",             tag: "TOR  " },
    ModuleMeta { index: 14, name: "kill_switch",             label: "Kill switch",             tag: "KILL " },
    ModuleMeta { index: 15, name: "i2p_integration",         label: "I2P Integration",         tag: "I2P  " },
    ModuleMeta { index: 16, name: "traffic_obfuscation",     label: "Traffic Obfuscation",     tag: "TRAFF" },
    ModuleMeta { index: 17, name: "sandbox_isolation",       label: "Sandbox Isolation",       tag: "JAIL " },
    ModuleMeta { index: 18, name: "dpi_spoofing",            label: "DPI Spoofing",            tag: "DPI  " },
    ModuleMeta { index: 19, name: "doh_shield",              label: "DoH Shield",              tag: "DOH  " },
    ModuleMeta { index: 20, name: "execution_stealth",       label: "Execution Stealth",       tag: "STLTH" },
    ModuleMeta { index: 21, name: "leak_auditor",            label: "Leak Auditor",            tag: "AUDIT" },
];

pub const NET_CAT_INDICES: [usize; 9] = [2, 3, 4, 13, 11, 15, 16, 18, 19];
pub const OBF_CAT_INDICES: [usize; 7] = [5, 6, 7, 8, 10, 17, 20];
pub const DEF_CAT_INDICES: [usize; 6] = [0, 1, 9, 12, 14, 21];

pub const EXEC_ORDER: [usize; 22] = [4, 6, 5, 8, 9, 12, 1, 10, 20, 13, 18, 17, 2, 3, 19, 15, 11, 16, 0, 14, 7, 21];
pub const STOP_ORDER: [usize; 22] = [21, 17, 14, 16, 0, 1, 2, 3, 19, 4, 5, 6, 7, 8, 9, 10, 11, 13, 18, 12, 15, 20];

pub fn profile_indices(name: &str) -> Option<&'static [usize]> {
    match name.to_ascii_lowercase().as_str() {
        "paranoia" | "0" => Some(&[0, 1, 2, 4, 5, 6, 7, 8, 12, 14, 16, 17, 18, 21]),
        "quick" | "1" => Some(&[2, 7, 17]),
        "pentest" | "2" => Some(&[0, 2, 4, 6, 8, 10, 11, 16]),
        "darknet" | "3" => Some(&[2, 13, 15]),
        "lockdown" | "4" => Some(&[0, 1, 9, 12, 14, 20]),
        "ghost" | "5" => Some(&[0, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 20, 21]),
        _ => None,
    }
}

pub trait AnonModule {
    fn name(&self) -> &'static str;
    fn start(&self, cfg: &AppConfig) -> Result<(), String>;
    fn stop(&self, cfg: &AppConfig) -> Result<(), String>;
    fn is_live(&self, cfg: &AppConfig) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_invariants_hold() {
        assert_eq!(ALL_MODULES.len(), 22);
        assert_eq!(
            NET_CAT_INDICES.len() + OBF_CAT_INDICES.len() + DEF_CAT_INDICES.len(),
            22
        );
        let mut seen_cat = [false; 22];
        for &i in NET_CAT_INDICES
            .iter()
            .chain(OBF_CAT_INDICES.iter())
            .chain(DEF_CAT_INDICES.iter())
        {
            assert!(!seen_cat[i], "duplicate category index {i}");
            seen_cat[i] = true;
        }
        assert!(seen_cat.iter().all(|&b| b));

        let mut seen_exec = [false; 22];
        let mut seen_stop = [false; 22];
        for &i in &EXEC_ORDER {
            assert!(!seen_exec[i]);
            seen_exec[i] = true;
        }
        for &i in &STOP_ORDER {
            assert!(!seen_stop[i]);
            seen_stop[i] = true;
        }
        assert!(seen_exec.iter().all(|&b| b));
        assert!(seen_stop.iter().all(|&b| b));
    }
}

