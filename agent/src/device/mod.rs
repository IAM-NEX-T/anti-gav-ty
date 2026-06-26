use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub ip_address: String,
    pub mac_address: String,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub first_seen: i64,
    pub last_seen: i64,
}

pub struct DeviceScanner {
    devices: HashMap<String, Device>,
}

impl DeviceScanner {
    pub fn new() -> Self {
        DeviceScanner {
            devices: HashMap::new(),
        }
    }

    pub fn scan(&mut self) -> Vec<Device> {
        let now = chrono::Utc::now().timestamp();
        let mut discovered = Vec::new();

        match self.read_arp_table() {
            Ok(entries) => {
                for (ip, mac) in entries {
                    let hostname = self.resolve_hostname(&ip);

                    let device = self.devices.entry(mac.clone()).or_insert_with(|| {
                        info!(ip = %ip, mac = %mac, "new device discovered");
                        Device {
                            ip_address: ip.clone(),
                            mac_address: mac.clone(),
                            hostname: None,
                            vendor: None,
                            first_seen: now,
                            last_seen: now,
                        }
                    });

                    device.ip_address = ip;
                    device.last_seen = now;
                    if hostname.is_some() {
                        device.hostname = hostname;
                    }

                    discovered.push(device.clone());
                }
            }
            Err(e) => {
                warn!("failed to read ARP table: {}", e);
            }
        }

        debug!(count = discovered.len(), "device scan complete");
        discovered
    }

    fn read_arp_table(&self) -> Result<Vec<(String, String)>, String> {
        if let Ok(contents) = std::fs::read_to_string("/proc/net/arp") {
            let mut entries = Vec::new();
            let mut lines = contents.lines();
            lines.next();

            for line in lines {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 4 {
                    let ip = fields[0].to_string();
                    let mac = fields[3].to_string();

                    if mac != "00:00:00:00:00:00"
                        && !ip.starts_with("127.")
                        && mac.len() == 17
                    {
                        entries.push((ip, mac));
                    }
                }
            }

            return Ok(entries);
        }

        let output = Command::new("ip")
            .args(["neigh", "show"])
            .output()
            .map_err(|e| format!("failed to run ip neigh: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut entries = Vec::new();

            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let ip = parts[0].to_string();
                    let mac = parts[4].to_string();

                    if mac != "00:00:00:00:00:00"
                        && !ip.starts_with("127.")
                        && mac.len() == 17
                        && parts[2] != "FAILED"
                    {
                        entries.push((ip, mac));
                    }
                }
            }

            return Ok(entries);
        }

        Err("no ARP data available".to_string())
    }

    fn resolve_hostname(&self, ip: &str) -> Option<String> {
        if let Ok(output) = Command::new("avahi-resolve-address")
            .arg(ip)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = stdout.split_whitespace().collect();
                if parts.len() >= 2 {
                    return Some(parts[1].trim_end_matches('.').to_string());
                }
            }
        }

        None
    }

    pub fn get_devices(&self) -> Vec<Device> {
        self.devices.values().cloned().collect()
    }
}
