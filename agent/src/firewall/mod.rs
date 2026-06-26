use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub id: String,
    pub device_mac: String,
    pub action: RuleAction,
    pub target: RuleTarget,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    Block,
    Allow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleTarget {
    pub ips: Vec<String>,
    pub domains: Vec<String>,
    pub ports: Vec<u16>,
}

pub struct Firewall {
    table_name: String,
    chain_name: String,
}

impl Firewall {
    pub fn new() -> Self {
        Firewall {
            table_name: "anti_gav_ty".to_string(),
            chain_name: "filter".to_string(),
        }
    }

    /// Initialize nftables table and chain
    pub fn init(&self) -> Result<(), String> {
        // Create table if it doesn't exist
        let output = Command::new("nft")
            .args(["add", "table", "inet", &self.table_name])
            .output()
            .map_err(|e| format!("failed to create table: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Table might already exist - that's ok
            if !stderr.contains("File exists") {
                warn!("nft table creation warning: {}", stderr.trim());
            }
        }

        // Create chain
        let output = Command::new("nft")
            .args([
                "add", "chain", "inet", &self.table_name, &self.chain_name,
                "{ type filter hook forward priority 0; }",
            ])
            .output()
            .map_err(|e| format!("failed to create chain: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("File exists") {
                warn!("nft chain creation warning: {}", stderr.trim());
            }
        }

        info!("firewall initialized: table={}, chain={}", self.table_name, self.chain_name);
        Ok(())
    }

    /// Block traffic from a specific MAC to specific IPs
    pub fn block_device_traffic(
        &self,
        rule_id: &str,
        device_mac: &str,
        target_ips: &[String],
    ) -> Result<(), String> {
        if target_ips.is_empty() {
            return Err("no target IPs specified".to_string());
        }

        let ip_set = target_ips
            .iter()
            .map(|ip| format!("ip daddr {}", ip))
            .collect::<Vec<_>>()
            .join(" ");

        let rule = format!(
            "ether saddr {} {} drop comment \"{}\"",
            device_mac, ip_set, rule_id
        );

        let output = Command::new("nft")
            .args([
                "add", "rule", "inet", &self.table_name, &self.chain_name,
                &rule,
            ])
            .output()
            .map_err(|e| format!("failed to add rule: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("nft error: {}", stderr.trim()));
        }

        info!(
            rule_id = %rule_id,
            device_mac = %device_mac,
            ips = ?target_ips,
            "firewall rule added: BLOCK"
        );

        Ok(())
    }

    /// Remove a rule by its comment/ID
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), String> {
        // List rules with handles, find the one with our comment, delete by handle
        let output = Command::new("nft")
            .args([
                "-a", "list", "chain", "inet", &self.table_name, &self.chain_name,
            ])
            .output()
            .map_err(|e| format!("failed to list rules: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Find the handle for our rule
        let mut handle = None;
        for line in stdout.lines() {
            if line.contains(rule_id) {
                // Handle is the number after "handle"
                if let Some(pos) = line.find("handle ") {
                    let rest = &line[pos + 7..];
                    if let Some(end) = rest.find(|c: char| !c.is_numeric()) {
                        handle = Some(rest[..end].to_string());
                    } else {
                        handle = Some(rest.trim().to_string());
                    }
                }
            }
        }

        match handle {
            Some(h) => {
                let output = Command::new("nft")
                    .args([
                        "delete", "rule", "inet", &self.table_name, &self.chain_name,
                        "handle", &h,
                    ])
                    .output()
                    .map_err(|e| format!("failed to delete rule: {}", e))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(format!("nft error: {}", stderr.trim()));
                }

                info!(rule_id = %rule_id, handle = %h, "firewall rule removed");
                Ok(())
            }
            None => {
                warn!(rule_id = %rule_id, "rule not found");
                Ok(()) // Not an error - already gone
            }
        }
    }

    /// Remove all rules for a specific device
    pub fn remove_device_rules(&self, device_mac: &str) -> Result<u32, String> {
        let output = Command::new("nft")
            .args([
                "-a", "list", "chain", "inet", &self.table_name, &self.chain_name,
            ])
            .output()
            .map_err(|e| format!("failed to list rules: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut removed = 0u32;

        // Collect handles for this device
        let mut handles: Vec<String> = Vec::new();
        for line in stdout.lines() {
            if line.contains(device_mac) {
                if let Some(pos) = line.find("handle ") {
                    let rest = &line[pos + 7..];
                    if let Some(end) = rest.find(|c: char| !c.is_numeric()) {
                        handles.push(rest[..end].to_string());
                    } else {
                        handles.push(rest.trim().to_string());
                    }
                }
            }
        }

        for handle in handles {
            let output = Command::new("nft")
                .args([
                    "delete", "rule", "inet", &self.table_name, &self.chain_name,
                    "handle", &handle,
                ])
                .output()
                .map_err(|e| format!("failed to delete rule: {}", e))?;

            if output.status.success() {
                removed += 1;
            }
        }

        info!(
            device_mac = %device_mac,
            count = removed,
            "removed device rules"
        );

        Ok(removed)
    }

    /// List all rules in our chain
    pub fn list_rules(&self) -> Result<String, String> {
        let output = Command::new("nft")
            .args(["list", "chain", "inet", &self.table_name, &self.chain_name])
            .output()
            .map_err(|e| format!("failed to list rules: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("nft error: {}", stderr.trim()))
        }
    }

    /// Clean up - remove our table and all rules
    pub fn cleanup(&self) -> Result<(), String> {
        let output = Command::new("nft")
            .args(["delete", "table", "inet", &self.table_name])
            .output()
            .map_err(|e| format!("failed to delete table: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("No such file or directory") {
                warn!("cleanup warning: {}", stderr.trim());
            }
        }

        info!("firewall cleaned up");
        Ok(())
    }
}
