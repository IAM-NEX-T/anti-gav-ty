package main

import (
	"flag"
	"fmt"
	"os"

	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/config"
	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/logger"
	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/server"
	"go.uber.org/zap"
)

func main() {
	configPath := flag.String("config", "configs/backend.yaml", "path to configuration file")
	flag.Parse()

	// Load configuration
	cfg, err := config.Load(*configPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to load config: %v\n", err)
		os.Exit(1)
	}

	// Initialize logger
	log, err := logger.New(cfg.Log.Level, cfg.Log.Format)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to initialize logger: %v\n", err)
		os.Exit(1)
	}
	defer log.Sync()

	log.Info("starting anti-gav-ty backend",
		zap.String("version", server.Version),
	)

	// Create and start server
	srv := server.New(log, server.Config{
		Host:         cfg.Server.Host,
		Port:         cfg.Server.Port,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
		IdleTimeout:  cfg.Server.IdleTimeout,
	})

	if err := srv.Start(); err != nil {
		log.Fatal("server error", zap.Error(err))
	}
}
