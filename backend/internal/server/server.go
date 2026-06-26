package server

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/IAM-NEX-T/anti-gav-ty/backend/internal/gateway"
	"github.com/gofiber/fiber/v2"
	"github.com/gofiber/fiber/v2/middleware/cors"
	"github.com/gofiber/fiber/v2/middleware/recover"
	"github.com/gofiber/fiber/v2/middleware/requestid"
	"go.uber.org/zap"
)

const (
	Version        = "0.1.0"
	ShutdownTimeout = 30 * time.Second
)

type Server struct {
	app    *fiber.App
	logger *zap.Logger
	config Config
}

type Config struct {
	Host         string
	Port         int
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
	IdleTimeout  time.Duration
}

// Handlers holds all route handlers.
type Handlers struct {
	Gateway *gateway.Handler
}

func New(logger *zap.Logger, cfg Config, handlers Handlers) *Server {
	app := fiber.New(fiber.Config{
		ReadTimeout:  cfg.ReadTimeout,
		WriteTimeout: cfg.WriteTimeout,
		IdleTimeout:  cfg.IdleTimeout,
		AppName:      "anti-gav-ty-backend",
	})

	app.Use(requestid.New())
	app.Use(recover.New())
	app.Use(cors.New(cors.Config{
		AllowOrigins: "*",
		AllowMethods: "GET,POST,PUT,DELETE,PATCH",
		AllowHeaders: "Content-Type,Authorization",
	}))

	s := &Server{
		app:    app,
		logger: logger,
		config: cfg,
	}

	s.registerRoutes(handlers)

	return s
}

func (s *Server) registerRoutes(h Handlers) {
	v1 := s.app.Group("/api/v1")

	v1.Get("/health", s.handleHealth)
	v1.Get("/health/ready", s.handleReady)

	// Gateway routes
	if h.Gateway != nil {
		h.Gateway.RegisterRoutes(v1)
	}
}

func (s *Server) handleHealth(c *fiber.Ctx) error {
	return c.JSON(fiber.Map{
		"status":  "ok",
		"version": Version,
	})
}

func (s *Server) handleReady(c *fiber.Ctx) error {
	return c.JSON(fiber.Map{
		"status": "ready",
	})
}

func (s *Server) Start() error {
	addr := fmt.Sprintf("%s:%d", s.config.Host, s.config.Port)

	errCh := make(chan error, 1)

	go func() {
		s.logger.Info("server starting",
			zap.String("host", s.config.Host),
			zap.Int("port", s.config.Port),
		)
		if err := s.app.Listen(addr); err != nil {
			errCh <- err
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-errCh:
		return fmt.Errorf("server listen error: %w", err)
	case sig := <-quit:
		s.logger.Info("shutdown signal received", zap.String("signal", sig.String()))
	}

	ctx, cancel := context.WithTimeout(context.Background(), ShutdownTimeout)
	defer cancel()

	s.logger.Info("shutting down server")
	if err := s.app.ShutdownWithContext(ctx); err != nil {
		return fmt.Errorf("shutdown error: %w", err)
	}

	s.logger.Info("server stopped gracefully")
	return nil
}
