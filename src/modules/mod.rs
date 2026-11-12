pub mod defense;
pub mod network;
pub mod obfuscation;
pub mod utils;

use crate::config::AppConfig;
use crate::registry::AnonModule;

pub fn get_module(name: &str) -> Option<Box<dyn AnonModule>> {
    match name {
        // Network & Routing
        "ip_changer" => Some(Box::new(network::IpChanger)),
        "dns_changer" => Some(Box::new(network::DnsChanger)),
        "doh_shield" => Some(Box::new(network::DohShield)),
        "kill_switch" => Some(Box::new(network::KillSwitch)),
        "tor_bridges" => Some(Box::new(network::TorBridges)),
        "dpi_spoofing" => Some(Box::new(network::DpiSpoofing)),
        "i2p_integration" => Some(Box::new(network::I2pIntegration)),
        "proxychains_integration" => Some(Box::new(network::ProxychainsIntegration)),
        "mac_changer" => Some(Box::new(network::MacChanger)),
        // System Obfuscation
        "timezone_changer" => Some(Box::new(obfuscation::TimezoneChanger)),
        "hostname_changer" => Some(Box::new(obfuscation::HostnameChanger)),
        "browser_anonymization" => Some(Box::new(obfuscation::BrowserAnonymization)),
        "os_obfuscation" => Some(Box::new(obfuscation::OsObfuscation)),
        "process_obfuscation" => Some(Box::new(obfuscation::ProcessObfuscation)),
        "sandbox_isolation" => Some(Box::new(obfuscation::SandboxIsolation)),
        "execution_stealth" => Some(Box::new(obfuscation::ExecutionStealth)),
        "traffic_obfuscation" => Some(Box::new(obfuscation::TrafficObfuscation)),
        // Active Defense
        "anti_mitm" => Some(Box::new(defense::AntiMitm)),
        "leak_auditor" => Some(Box::new(defense::LeakAuditor)),
        "volatile_logging" => Some(Box::new(defense::VolatileLogging)),
        "bluetooth_disabler" => Some(Box::new(defense::BluetoothDisabler)),
        "anti_cold_boot" => Some(Box::new(defense::AntiColdBoot)),
        _ => None,
    }
}

pub fn start_module(cfg: &AppConfig, name: &str) -> Result<(), String> {
    if let Some(m) = get_module(name) {
        if m.name() != name {
            return Err(format!("Module registry mismatch: {} != {name}", m.name()));
        }
        m.start(cfg)
    } else {
        Err(format!("Unknown module: {name}"))
    }
}

pub fn stop_module(cfg: &AppConfig, name: &str) -> Result<(), String> {
    if let Some(m) = get_module(name) {
        if m.name() != name {
            return Err(format!("Module registry mismatch: {} != {name}", m.name()));
        }
        m.stop(cfg)
    } else {
        Err(format!("Unknown module: {name}"))
    }
}

pub fn is_module_live(cfg: &AppConfig, name: &str) -> bool {
    get_module(name)
        .map(|m| m.name() == name && m.is_live(cfg))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ALL_MODULES;

    #[test]
    fn all_registry_modules_resolve_and_match_names() {
        for meta in ALL_MODULES {
            let m = get_module(meta.name).expect("module must be registered in get_module");
            assert_eq!(m.name(), meta.name);
        }
        assert!(get_module("does_not_exist").is_none());
    }
}
