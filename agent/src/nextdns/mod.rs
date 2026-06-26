use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQuery {
    pub domain: String,
    pub device_name: String,
    pub device_mac: String,
    pub timestamp: i64,
    pub resolved_ips: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct NextDnsLogResponse {
    data: Vec<NextDnsLogEntry>,
}

#[derive(Debug, Deserialize)]
struct NextDnsLogEntry {
    domain: String,
    root: String,
    #[serde(rename = "type")]
    query_type: String,
    timestamp: String,
    device: Option<NextDnsDevice>,
    client: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NextDnsDevice {
    id: String,
    name: String,
    model: String,
}

pub struct NextDnsClient {
    api_key: String,
    profile_id: String,
    http_client: reqwest::Client,
    // Known game server patterns
    game_patterns: HashMap<&'static str, &'static str>,
}

impl NextDnsClient {
    pub fn new(api_key: String, profile_id: String) -> Self {
        let mut game_patterns = HashMap::new();
        game_patterns.insert("rockstargames.com", "GTA Online");
        game_patterns.insert("ros.rockstargames.com", "GTA Online");
        game_patterns.insert("socialclub.rockstargames.com", "GTA Social Club");
        game_patterns.insert("psn.com", "PlayStation Network");
        game_patterns.insert("playstation.net", "PlayStation Network");
        game_patterns.insert("xboxlive.com", "Xbox Live");
        game_patterns.insert("fortnite.com", "Fortnite");
        game_patterns.insert("epicgames.com", "Epic Games");
        game_patterns.insert("callofduty.com", "Call of Duty");
        game_patterns.insert("activision.com", "Activision");
        game_patterns.insert("ea.com", "EA Games");
        game_patterns.insert("steam.com", "Steam");

        NextDnsClient {
            api_key,
            profile_id,
            http_client: reqwest::Client::new(),
            game_patterns,
        }
    }

    /// Fetch recent DNS queries from NextDNS
    pub async fn fetch_logs(&self) -> Result<Vec<DnsQuery>, String> {
        let url = format!(
            "https://api.nextdns.io/profiles/{}/logs?limit=100",
            self.profile_id
        );

        let resp = self
            .http_client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("NextDNS API request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("NextDNS API error {}: {}", status, body));
        }

        let logs: NextDnsLogResponse = resp
            .json()
            .await
            .map_err(|e| format!("failed to parse NextDNS response: {}", e))?;

        let queries: Vec<DnsQuery> = logs
            .data
            .into_iter()
            .map(|entry| {
                let device_name = entry
                    .device
                    .as_ref()
                    .map(|d| d.name.clone())
                    .unwrap_or_else(|| entry.client.unwrap_or_else(|| "unknown".to_string()));

                let device_mac = entry
                    .device
                    .as_ref()
                    .map(|d| d.id.clone())
                    .unwrap_or_else(|| "00:00:00:00:00:00".to_string());

                let timestamp = entry
                    .timestamp
                    .parse::<i64>()
                    .unwrap_or(0);

                DnsQuery {
                    domain: entry.domain,
                    device_name,
                    device_mac,
                    timestamp,
                    resolved_ips: Vec::new(), // Filled by separate analytics call
                }
            })
            .collect();

        debug!(count = queries.len(), "fetched NextDNS logs");
        Ok(queries)
    }

    /// Identify which games are being played based on DNS queries
    pub fn identify_games(&self, queries: &[DnsQuery]) -> Vec<GameActivity> {
        let mut activities: Vec<GameActivity> = Vec::new();
        let mut seen: HashMap<String, bool> = HashMap::new();

        for query in queries {
            for (pattern, game_name) in &self.game_patterns {
                if query.domain.contains(pattern) {
                    let key = format!("{}:{}", query.device_mac, game_name);
                    if !seen.contains_key(&key) {
                        seen.insert(key, true);
                        activities.push(GameActivity {
                            device_name: query.device_name.clone(),
                            device_mac: query.device_mac.clone(),
                            game: game_name.to_string(),
                            domain: query.domain.clone(),
                            last_seen: query.timestamp,
                        });
                    }
                }
            }
        }

        activities
    }

    /// Get resolved IPs for domains (using NextDNS analytics)
    pub async fn get_resolved_ips(&self, domain: &str) -> Result<Vec<String>, String> {
        let url = format!(
            "https://api.nextdns.io/profiles/{}/analytics/domains?domain={}",
            self.profile_id, domain
        );

        let resp = self
            .http_client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("NextDNS analytics request failed: {}", e))?;

        if !resp.status().is_success() {
            return Ok(Vec::new()); // Not critical, return empty
        }

        // Simplified - in production, parse actual response for IPs
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GameActivity {
    pub device_name: String,
    pub device_mac: String,
    pub game: String,
    pub domain: String,
    pub last_seen: i64,
}
