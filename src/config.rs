use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const ANON_VERSION: &str = "3.0.0";

pub const DEFAULT_SNOWFLAKE_URL: &str = "https://1098762253.rsc.cdn77.org/";
pub const DEFAULT_SNOWFLAKE_FRONTS: &str = "www.cdn77.com,www.phpmyadmin.net";
pub const DEFAULT_SNOWFLAKE_ICE: &str = "stun:stun.l.google.com:19302,stun:stun.antisip.com:3478,stun:stun.bluesip.net:3478,stun:stun.dus.net:3478,stun:stun.epygi.com:3478,stun:stun.sonetel.com:3478,stun:stun.uls.co.za:3478,stun:stun.voipgate.com:3478,stun:stun.voys.nl:3478";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub base_dir: PathBuf,
    pub src_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub state_file: PathBuf,
    pub torrc: PathBuf,
    pub hosts_file: PathBuf,
    pub resolv_conf: PathBuf,
    pub proxychains_conf: PathBuf,
    pub apparmor_tor_profile: PathBuf,
    pub netns_etc_dir: PathBuf,
    pub wrapper_dir: PathBuf,
    pub snowflake_url: String,
    pub snowflake_fronts: String,
    pub snowflake_ice: String,
    pub tor_bootstrap_timeout: Option<u64>,
    pub audit_interval_secs: u64,
    pub mitm_poll_interval_secs: u64,
    pub traffic_chaff_url: String,
    pub volatile_log_size: String,
}

impl AppConfig {
    pub fn load() -> Result<Self, String> {
        let base_dir = resolve_base_dir()?;
        let src_dir = base_dir.join("assets");

        let persist_dir = env::var_os("ANON_PERSIST_DIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| base_dir.clone());

        let backup_dir = env::var_os("BACKUPDIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| persist_dir.join("backups"));

        let _ = fs::create_dir_all(&backup_dir);

        let state_file = env::var_os("ANON_STATE_FILE")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| backup_dir.join("anon.state"));

        let torrc = env_path("TORRC", "/etc/tor/torrc");
        let hosts_file = env_path("ANON_HOSTS_FILE", "/etc/hosts");
        let resolv_conf = env_path("ANON_RESOLV_CONF", "/etc/resolv.conf");
        let proxychains_conf = env_path("ANON_PROXYCHAINS_CONF", "/etc/proxychains4.conf");
        let apparmor_tor_profile =
            env_path("APPARMOR_TOR_PROFILE", "/etc/apparmor.d/usr.bin.tor.anon");
        let netns_etc_dir = env_path("NETNS_ETC_DIR", "/etc/netns/anon_jail");
        let wrapper_dir = env_path("WRAPPER_DIR", "/usr/local/bin");

        let snowflake_url = env_str("ANON_SNOWFLAKE_URL", DEFAULT_SNOWFLAKE_URL);
        let snowflake_fronts = env_str("ANON_SNOWFLAKE_FRONTS", DEFAULT_SNOWFLAKE_FRONTS);
        let snowflake_ice = env_str("ANON_SNOWFLAKE_ICE", DEFAULT_SNOWFLAKE_ICE);

        let tor_bootstrap_timeout = env::var("ANON_TOR_BOOTSTRAP_TIMEOUT")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok());

        let audit_interval_secs = env::var("ANON_AUDIT_INTERVAL")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(15);

        let mitm_poll_interval_secs = env::var("ANON_MITM_POLL_INTERVAL")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|&v| v > 0)
            .unwrap_or(2);

        let traffic_chaff_url = env_str(
            "ANON_TRAFFIC_CHAFF_URL",
            "https://check.torproject.org/api/ip",
        );
        let volatile_log_size = env_str("ANON_VOLATILE_LOG_SIZE", "100M");

        Ok(Self {
            base_dir,
            src_dir,
            backup_dir,
            state_file,
            torrc,
            hosts_file,
            resolv_conf,
            proxychains_conf,
            apparmor_tor_profile,
            netns_etc_dir,
            wrapper_dir,
            snowflake_url,
            snowflake_fronts,
            snowflake_ice,
            tor_bootstrap_timeout,
            audit_interval_secs,
            mitm_poll_interval_secs,
            traffic_chaff_url,
            volatile_log_size,
        })
    }
}

fn env_path(key: &str, default: &str) -> PathBuf {
    env::var_os(key)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn env_str(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub fn resolve_base_dir() -> Result<PathBuf, String> {
    if let Some(bd) = env::var_os("BASE_DIR").filter(|v| !v.is_empty()) {
        let p = PathBuf::from(bd);
        if p.join("assets").is_dir() {
            return Ok(p);
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if exe_dir.join("assets").is_dir() {
                return Ok(exe_dir.to_path_buf());
            }
            if let Some(parent) = exe_dir.parent() {
                if parent.join("assets").is_dir() {
                    return Ok(parent.to_path_buf());
                }
            }
        }
    }

    if let Ok(cwd) = env::current_dir() {
        if cwd.join("assets").is_dir() {
            return Ok(cwd);
        }
    }

    let usr_share = Path::new("/usr/share/anon");
    if usr_share.join("assets").is_dir() {
        return Ok(usr_share.to_path_buf());
    }

    Err("Installation directory not found!".to_string())
}

