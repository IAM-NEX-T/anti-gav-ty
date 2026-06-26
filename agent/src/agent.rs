use crate::config::Config;
use crate::device::DeviceScanner;
use crate::error::AgentError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info};

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

pub struct Agent {
    config: Config,
    client: Client,
    mac_address: String,
    hostname: String,
    scanner: DeviceScanner,
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
        })
    }

    pub async fn run(&mut self) -> Result<(), AgentError> {
        self.register().await?;

        let heartbeat_interval = Duration::from_secs(self.config.agent.heartbeat_interval_secs);
        let mut hb_ticker = tokio::time::interval(heartbeat_interval);
        let mut scan_ticker = tokio::time::interval(Duration::from_secs(60));

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
}
