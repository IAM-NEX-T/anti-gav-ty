CREATE TABLE IF NOT EXISTS gateways (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        VARCHAR(255) NOT NULL,
    mac_address VARCHAR(17) NOT NULL UNIQUE,
    ip_address  VARCHAR(45),
    hostname    VARCHAR(255),
    version     VARCHAR(50) NOT NULL DEFAULT '0.1.0',
    status      VARCHAR(20) NOT NULL DEFAULT 'offline',
    last_seen   TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_gateways_status ON gateways(status);
CREATE INDEX idx_gateways_last_seen ON gateways(last_seen);
