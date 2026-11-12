use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::config::AppConfig;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En = 0,
    Tr = 1,
}

static ACTIVE_LANG: AtomicU8 = AtomicU8::new(0);

pub fn current_lang() -> Lang {
    match ACTIVE_LANG.load(Ordering::Relaxed) {
        1 => Lang::Tr,
        _ => Lang::En,
    }
}

pub fn set_lang(lang: Lang) {
    ACTIVE_LANG.store(lang as u8, Ordering::Relaxed);
}

#[inline]
pub fn t<'a>(en_str: &'a str, tr_str: &'a str) -> &'a str {
    if current_lang() == Lang::Tr {
        return tr_str;
    }
    en_str
}

pub fn lang_file_path(cfg: &AppConfig) -> PathBuf {
    cfg.backup_dir.join("anon.lang")
}

pub fn parse_lang(raw: &str) -> Option<Lang> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "tr" | "turkish" | "türkçe" | "turkce" => Some(Lang::Tr),
        "en" | "english" | "ingilizce" => Some(Lang::En),
        _ => None,
    }
}

/// Initializes interface language from `ANON_LANG` env var or persisted `anon.lang` file.
/// Defaults strictly to English (`Lang::En`) when unset or unrecognized.
pub fn init(cfg: &AppConfig) {
    if let Ok(env_val) = std::env::var("ANON_LANG") {
        if let Some(lang) = parse_lang(&env_val) {
            set_lang(lang);
            return;
        }
    }
    let path = lang_file_path(cfg);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Some(lang) = parse_lang(&content) {
            set_lang(lang);
            return;
        }
    }
    set_lang(Lang::En);
}

pub fn save_lang(cfg: &AppConfig, lang: Lang) -> Result<(), String> {
    let path = lang_file_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all: {e}"))?;
    }
    let code = match lang {
        Lang::En => "en\n",
        Lang::Tr => "tr\n",
    };
    fs::write(&path, code).map_err(|e| format!("write anon.lang: {e}"))?;
    set_lang(lang);
    Ok(())
}

pub fn module_label<'a>(mod_name: &str, default_en: &'a str) -> &'a str {
    if current_lang() == Lang::En {
        return default_en;
    }
    match mod_name {
        "ip_changer" => "IP Değiştirici (Tor)",
        "dns_changer" => "DNS Değiştirici",
        "doh_shield" => "DoH Kalkanı",
        "kill_switch" => "Ağ Kilidi (Kill Switch)",
        "tor_bridges" => "Tor Köprüleri (obfs4)",
        "dpi_spoofing" => "DPI Atlama (Snowflake)",
        "i2p_integration" => "I2P Ağı Entegrasyonu",
        "proxychains_integration" => "Proxychains4 Zinciri",
        "mac_changer" => "MAC Adresi Değiştirici",
        "timezone_changer" => "Saat Dilimi Değiştirici",
        "hostname_changer" => "Cihaz Adı (Hostname)",
        "browser_anonymization" => "Tarayıcı Anonimleştirme",
        "os_obfuscation" => "İşletim Sistemi İz Gizleme",
        "process_obfuscation" => "Süreç Gizleme (hidepid)",
        "sandbox_isolation" => "Sandbox İzolasyonu (Jail)",
        "execution_stealth" => "Çalışma Gizliliği (RAM)",
        "traffic_obfuscation" => "Trafik Karartma (Chaff)",
        "anti_mitm" => "ARP/MITM Koruması",
        "leak_auditor" => "Sızıntı Denetçisi",
        "volatile_logging" => "Uçucu RAM Günlükleri",
        "bluetooth_disabler" => "Bluetooth Devre Dışı",
        "anti_cold_boot" => "Cold-Boot Bellek Kilidi",
        _ => default_en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ALL_MODULES;

    #[test]
    fn i18n_defaults_en_and_persists_lang() {
        let dir = std::env::temp_dir().join(format!("anon_i18n_test_{}", crate::helpers::rand_u64()));
        fs::create_dir_all(&dir).unwrap();
        let mut cfg = AppConfig::load().unwrap();
        cfg.backup_dir = dir.clone();

        set_lang(Lang::En);
        assert_eq!(t("Quit", "Çıkış"), "Quit");

        save_lang(&cfg, Lang::Tr).unwrap();
        assert_eq!(current_lang(), Lang::Tr);
        assert_eq!(t("Quit", "Çıkış"), "Çıkış");

        set_lang(Lang::En);
        init(&cfg);
        assert_eq!(current_lang(), Lang::Tr);

        for m in ALL_MODULES {
            assert_ne!(module_label(m.name, m.label), "");
            assert_ne!(module_label(m.name, m.label), m.label);
        }

        save_lang(&cfg, Lang::En).unwrap();
        assert_eq!(current_lang(), Lang::En);
        let _ = fs::remove_dir_all(&dir);
    }
}
