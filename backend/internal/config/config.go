package config

import (
	"fmt"
	"os"
	"strconv"
	"time"

	"gopkg.in/yaml.v3"
)

// Config represents the top-level application configuration.
type Config struct {
	Server   ServerConfig   `yaml:"server"`
	Log      LogConfig      `yaml:"log"`
	Database DatabaseConfig `yaml:"database"`
	NextDNS  NextDNSConfig  `yaml:"nextdns"`
}

// ServerConfig holds HTTP server settings.
type ServerConfig struct {
	Host         string        `yaml:"host"`
	Port         int           `yaml:"port"`
	ReadTimeout  time.Duration `yaml:"read_timeout"`
	WriteTimeout time.Duration `yaml:"write_timeout"`
	IdleTimeout  time.Duration `yaml:"idle_timeout"`
}

// LogConfig holds logging settings.
type LogConfig struct {
	Level  string `yaml:"level"`
	Format string `yaml:"format"`
}

// DatabaseConfig holds database connection settings.
type DatabaseConfig struct {
	URL             string        `yaml:"url"`
	MaxOpenConns    int           `yaml:"max_open_conns"`
	MaxIdleConns    int           `yaml:"max_idle_conns"`
	ConnMaxLifetime time.Duration `yaml:"conn_max_lifetime"`
}

// NextDNSConfig holds NextDNS API settings.
type NextDNSConfig struct {
	APIKey    string `yaml:"api_key"`
	ProfileID string `yaml:"profile_id"`
}

// Load reads the configuration file and applies environment variable overrides.
func Load(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("reading config file: %w", err)
	}

	var cfg Config
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("parsing config file: %w", err)
	}

	applyEnvOverrides(&cfg)

	return &cfg, nil
}

// applyEnvOverrides checks for environment variables with the LAMO_ prefix
// and overrides the corresponding config values.
func applyEnvOverrides(cfg *Config) {
	if v := os.Getenv("LAMO_SERVER_HOST"); v != "" {
		cfg.Server.Host = v
	}
	if v := os.Getenv("LAMO_SERVER_PORT"); v != "" {
		if port, err := strconv.Atoi(v); err == nil {
			cfg.Server.Port = port
		}
	}
	if v := os.Getenv("LAMO_LOG_LEVEL"); v != "" {
		cfg.Log.Level = v
	}
	if v := os.Getenv("LAMO_LOG_FORMAT"); v != "" {
		cfg.Log.Format = v
	}
	if v := os.Getenv("LAMO_DATABASE_URL"); v != "" {
		cfg.Database.URL = v
	}
	if v := os.Getenv("LAMO_NEXTDNS_API_KEY"); v != "" {
		cfg.NextDNS.APIKey = v
	}
	if v := os.Getenv("LAMO_NEXTDNS_PROFILE_ID"); v != "" {
		cfg.NextDNS.ProfileID = v
	}
}
