use crate::config::Config;
use crate::device::DeviceScanner;
use crate::error::AgentError;
use crate::firewall::Firewall;
use crate::nextdns::NextDnsClient;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Serialize)]
struct RegisterRequest {
    name: String,
    mac_address: String,
    ip_address: String,
    hostname: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    gateway: GatewayInfo,
}

#[derive(Debug, Deserialize)]
struct GatewayInfo {
    id: String,
    status: String,
}

#[derive(Debug, Serialize)]
struct HeartbeatRequest {
    mac_address: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct DevicesReport {
    gateway_mac: String,
    devices: Vec<crate::device::Device>,
}

#[derive(Debug, Serialize)]
struct GameActivityReport {
    gateway_mac: String,
    activities: Vec<crate::nextdns::GameActivity>,
}

#[derive(Debug, Deserialize)]
struct PolicyUpdate {
    rules: Vec<FirewallCommand>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
enum FirewallCommand {
    #[serde(rename = "block")]
    Block {
        rule_id: String,
        device_mac: String,
        target_ips: Vec<String>,
    },
    #[serde(rename = "unblock")]
    Unblock {
        rule_id: String,
    },
    #[serde(rename = "unblock_device")]
    UnblockDevice {
        device_mac: String,
    },
}

pub struct Agent {
    config: Config,
    client: Client,
    mac_address: String,
    hostname: String,
    scanner: DeviceScanner,
    firewall: Firewall,
    nextdns: Option<NextDnsClient>,
}

impl Agent {
    pub async fn new(config: Config) -> Result<Self, AgentError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let mac = mac_address::get_mac_address()
            .ok()
            .flatten()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "00:00:00:00:00:00".to_string());

        let host = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());

        let firewall = Firewall::new();
        if let Err(e) = firewall.init() {
            warn!("firewall init error (may need root): {}", e);
        }

        // Initialize NextDNS client if configured
        let nextdns = if !config.nextdns.api_key.is_empty() && !config.nextdns.profile_id.is_empty() {
            info!("NextDNS integration enabled");
            Some(NextDnsClient::new(
                config.nextdns.api_key.clone(),
                config.nextdns.profile_id.clone(),
            ))
        } else {
            warn!("NextDNS not configured - game detection disabled");
            None
        };

        info!(
            mac_address = %mac,
            hostname = %host,
            "agent initialized"
        );

        Ok(Agent {
            config,
            client,
            mac_address: mac,
            hostname: host,
            scanner: DeviceScanner::new(),
            firewall,
            nextdns,
        })
    }

    pub async fn run(&mut self) -> Result<(), AgentError> {
        self.register().await?;

        let heartbeat_interval = Duration::from_secs(self.config.agent.heartbeat_interval_secs);
        let mut hb_ticker = tokio::time::interval(heartbeat_interval);
        let mut scan_ticker = tokio::time::interval(Duration::from_secs(60));
        let mut policy_ticker = tokio::time::interval(Duration::from_secs(10));
        let mut nextdns_ticker = tokio::time::interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = hb_ticker.tick() => {
                    if let Err(e) = self.heartbeat().await {
                        error!("heartbeat failed: {}", e);
                        self.register().await?;
                    }
                }
                _ = scan_ticker.tick() => {
                    let devices = self.scanner.scan();
                    if !devices.is_empty() {
                        info!(count = devices.len(), "devices found");
                        if let Err(e) = self.report_devices(&devices).await {
                            error!("failed to report devices: {}", e);
                        }
                    }
                }
                _ = policy_ticker.tick() => {
                    if let Err(e) = self.poll_policies().await {
                        error!("failed to poll policies: {}", e);
                    }
                }
                _ = nextdns_ticker.tick() => {
                    if let Some(ref nextdns) = self.nextdns {
                        match nextdns.fetch_logs().await {
                            Ok(queries) => {
                                let activities = nextdns.identify_games(&queries);
                                if !activities.is_empty() {
                                    info!(count = activities.len(), "game activity detected");
                                    if let Err(e) = self.report_game_activity(&activities).await {
                                        error!("failed to report game activity: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("NextDNS fetch failed: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    async fn register(&mut self) -> Result<(), AgentError> {
        let url = format!(
            "{}{}",
            self.config.backend.url, self.config.backend.register_path
        );

        let ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "0.0.0.0".to_string());

        let req = RegisterRequest {
            name: self.config.agent.name.clone(),
            mac_address: self.mac_address.clone(),
            ip_address: ip,
            hostname: self.hostname.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        info!(url = %url, "registering with backend");

        let resp = self.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AgentError::Registration(format!(
                "backend returned {}: {}",
                status, body
            )));
        }

        let reg: RegisterResponse = resp.json().await?;
        info!(
            gateway_id = %reg.gateway.id,
            status = %reg.gateway.status,
            "registered successfully"
        );

        Ok(())
    }

    async fn heartbeat(&self) -> Result<(), AgentError> {
        let url = format!(
            "{}{}",
            self.config.backend.url, self.config.backend.heartbeat_path
        );

        let req = HeartbeatRequest {
            mac_address: self.mac_address.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let resp = self.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AgentError::Heartbeat(format!(
                "backend returned {}: {}",
                status, body
            )));
        }

        tracing::debug!("heartbeat sent");
        Ok(())
    }

    async fn report_devices(&self, devices: &[crate::device::Device]) -> Result<(), AgentError> {
        let url = format!("{}/api/v1/devices/report", self.config.backend.url);

        let report = DevicesReport {
            gateway_mac: self.mac_address.clone(),
            devices: devices.to_vec(),
        };

        let resp = self.client.post(&url).json(&report).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AgentError::Heartbeat(format!(
                "device report failed: {} - {}",
                status, body
            )));
        }

        tracing::debug!(count = devices.len(), "devices reported");
        Ok(())
    }

    async fn report_game_activity(
        &self,
        activities: &[crate::nextdns::GameActivity],
    ) -> Result<(), AgentError> {
        let url = format!("{}/api/v1/game-activity/report", self.config.backend.url);

        let report = GameActivityReport {
            gateway_mac: self.mac_address.clone(),
            activities: activities.to_vec(),
        };

        let resp = self.client.post(&url).json(&report).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AgentError::Heartbeat(format!(
                "game activity report failed: {} - {}",
                status, body
            )));
        }

        tracing::debug!(count = activities.len(), "game activity reported");
        Ok(())
    }

    async fn poll_policies(&self) -> Result<(), AgentError> {
        let url = format!(
            "{}/api/v1/gateways/{}/policies",
            self.config.backend.url, self.mac_address
        );

        let resp = self.client.get(&url).send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AgentError::Heartbeat(format!(
                "policy poll failed: {} - {}",
                status, body
            )));
        }

        let update: PolicyUpdate = resp.json().await?;

        for cmd in &update.rules {
            self.apply_policy(cmd);
        }

        Ok(())
    }

    fn apply_policy(&self, cmd: &FirewallCommand) {
        match cmd {
            FirewallCommand::Block {
                rule_id,
                device_mac,
                target_ips,
            } => {
                match self.firewall.block_device_traffic(rule_id, device_mac, target_ips) {
                    Ok(()) => info!(rule_id = %rule_id, "BLOCK applied"),
                    Err(e) => error!(rule_id = %rule_id, error = %e, "BLOCK failed"),
                }
            }
            FirewallCommand::Unblock { rule_id } => {
                match self.firewall.remove_rule(rule_id) {
                    Ok(()) => info!(rule_id = %rule_id, "UNBLOCK applied"),
                    Err(e) => error!(rule_id = %rule_id, error = %e, "UNBLOCK failed"),
                }
            }
            FirewallCommand::UnblockDevice { device_mac } => {
                match self.firewall.remove_device_rules(device_mac) {
                    Ok(count) => info!(device_mac = %device_mac, count = count, "device unblocked"),
                    Err(e) => error!(device_mac = %device_mac, error = %e, "device unblock failed"),
                }
            }
        }
    }
}
