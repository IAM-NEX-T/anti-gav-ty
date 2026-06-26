const API_BASE = 'http://localhost:8080/api/v1';

export interface Gateway {
  id: string;
  name: string;
  mac_address: string;
  ip_address: string;
  hostname: string;
  version: string;
  status: 'online' | 'offline';
  last_seen: string;
}

export interface Device {
  ip_address: string;
  mac_address: string;
  hostname?: string;
  vendor?: string;
  first_seen: number;
  last_seen: number;
}

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(`${API_BASE}${url}`);
  if (!res.ok) throw new Error(`API error: ${res.status}`);
  return res.json();
}

export async function getGateways(): Promise<Gateway[]> {
  const data = await fetchJson<{ gateways: Gateway[] }>('/gateways');
  return data.gateways;
}

export async function getGateway(mac: string): Promise<Gateway> {
  const data = await fetchJson<{ gateway: Gateway }>(`/gateways/${encodeURIComponent(mac)}`);
  return data.gateway;
}

export async function blockDevice(deviceMac: string, targetIps: string[], ruleId: string) {
  const res = await fetch(`${API_BASE}/policies/block`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      device_mac: deviceMac,
      target_ips: targetIps,
      rule_id: ruleId,
    }),
  });
  if (!res.ok) throw new Error(`Block failed: ${res.status}`);
  return res.json();
}

export async function unblockDevice(ruleId: string) {
  const res = await fetch(`${API_BASE}/policies/unblock`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ rule_id: ruleId }),
  });
  if (!res.ok) throw new Error(`Unblock failed: ${res.status}`);
  return res.json();
}
