package config

import (
	"fmt"
	"os"
	"strconv"
	"time"

	"gopkg.in/yaml.v3"
)

type Config struct {
	Server     ServerConfig     `yaml:"server"`
	Log        LogConfig        `yaml:"log"`
	Database   DatabaseConfig   `yaml:"database"`
	NextDNS    NextDNSConfig    `yaml:"nextdns"`
	Migrations MigrationsConfig `yaml:"migrations"`
}

type ServerConfig struct {
	Host         string        `yaml:"host"`
	Port         int           `yaml:"port"`
	ReadTimeout  time.Duration `yaml:"read_timeout"`
	WriteTimeout time.Duration `yaml:"write_timeout"`
	IdleTimeout  time.Duration `yaml:"idle_timeout"`
}

type LogConfig struct {
	Level  string `yaml:"level"`
	Format string `yaml:"format"`
}

type DatabaseConfig struct {
	URL             string        `yaml:"url"`
	MaxOpenConns    int           `yaml:"max_open_conns"`
	MaxIdleConns    int           `yaml:"max_idle_conns"`
	ConnMaxLifetime time.Duration `yaml:"conn_max_lifetime"`
}

type NextDNSConfig struct {
	APIKey    string `yaml:"api_key"`
	ProfileID string `yaml:"profile_id"`
}

type MigrationsConfig struct {
	Path string `yaml:"path"`
}

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

	if cfg.Migrations.Path == "" {
		cfg.Migrations.Path = "migrations"
	}

	return &cfg, nil
}

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
