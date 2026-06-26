package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"time"

	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/config"
	dbpkg "github.com/IAM-NEX-T/anti-gav-ty/backend/internal/database"
	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/gateway"
	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/logger"
	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/server"
	"go.uber.org/zap"
)

func main() {
	configPath := flag.String("config", "configs/backend.yaml", "path to configuration file")
	flag.Parse()

	cfg, err := config.Load(*configPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to load config: %v\n", err)
		os.Exit(1)
	}

	log, err := logger.New(cfg.Log.Level, cfg.Log.Format)
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to initialize logger: %v\n", err)
		os.Exit(1)
	}
	defer log.Sync()

	log.Info("starting anti-gav-ty backend",
		zap.String("version", server.Version),
	)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	db, err := dbpkg.New(ctx, log, dbpkg.Config{
		URL:             cfg.Database.URL,
		MaxOpenConns:    cfg.Database.MaxOpenConns,
		MaxIdleConns:    cfg.Database.MaxIdleConns,
		ConnMaxLifetime: cfg.Database.ConnMaxLifetime,
	})
	if err != nil {
		log.Fatal("failed to connect to database", zap.Error(err))
	}
	defer db.Close()

	if err := dbpkg.RunMigrations(log, cfg.Database.URL, cfg.Migrations.Path); err != nil {
		log.Fatal("failed to run migrations", zap.Error(err))
	}

	// Initialize repositories and handlers
	gatewayRepo := gateway.NewRepository(db.Pool, log)
	gatewayHandler := gateway.NewHandler(gatewayRepo, log)

	srv := server.New(log, server.Config{
		Host:         cfg.Server.Host,
		Port:         cfg.Server.Port,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
		IdleTimeout:  cfg.Server.IdleTimeout,
	}, server.Handlers{
		Gateway: gatewayHandler,
	})

	if err := srv.Start(); err != nil {
		log.Fatal("server error", zap.Error(err))
	}
}
