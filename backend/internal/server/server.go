package server

import (
	"context"
	"fmt"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/gofiber/fiber/v2/middleware/cors"
	"github.com/gofiber/fiber/v2/middleware/recover"
	"github.com/gofiber/fiber/v2/middleware/requestid"
	"go.uber.org/zap"
)

const (
	// Version is the current API version.
	Version = "0.1.0"
	// ShutdownTimeout is the maximum time to wait for graceful shutdown.
	ShutdownTimeout = 30 * time.Second
)

// Server wraps the Fiber HTTP server and its dependencies.
type Server struct {
	app    *fiber.App
	logger *zap.Logger
	config Config
}

// Config holds the server configuration.
type Config struct {
	Host         string
	Port         int
	ReadTimeout  time.Duration
	WriteTimeout time.Duration
	IdleTimeout  time.Duration
}

// New creates a new Server instance.
func New(logger *zap.Logger, cfg Config) *Server {
	app := fiber.New(fiber.Config{
		ReadTimeout:  cfg.ReadTimeout,
		WriteTimeout: cfg.WriteTimeout,
		IdleTimeout:  cfg.IdleTimeout,
		AppName:      "anti-gav-ty-backend",
	})

	// Middleware
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

	s.registerRoutes()

	return s
}

// registerRoutes sets up all API routes.
func (s *Server) registerRoutes() {
	// API v1 group
	v1 := s.app.Group("/api/v1")

	// Health check
	v1.Get("/health", s.handleHealth)

	// Readiness check
	v1.Get("/health/ready", s.handleReady)
}

// handleHealth returns basic health information.
func (s *Server) handleHealth(c *fiber.Ctx) error {
	return c.JSON(fiber.Map{
		"status":  "ok",
		"version": Version,
	})
}

// handleReady returns the readiness state.
func (s *Server) handleReady(c *fiber.Ctx) error {
	return c.JSON(fiber.Map{
		"status": "ready",
	})
}

// Start begins listening and blocks until the server shuts down.
func (s *Server) Start() error {
	addr := fmt.Sprintf("%s:%d", s.config.Host, s.config.Port)

	// Channel to receive errors from the listen goroutine
	errCh := make(chan error, 1)

	// Start server in goroutine
	go func() {
		s.logger.Info("server starting",
			zap.String("host", s.config.Host),
			zap.Int("port", s.config.Port),
		)
		if err := s.app.Listen(addr); err != nil {
			errCh <- err
		}
	}()

	// Wait for interrupt signal or listen error
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-errCh:
		return fmt.Errorf("server listen error: %w", err)
	case sig := <-quit:
		s.logger.Info("shutdown signal received", zap.String("signal", sig.String()))
	}

	// Graceful shutdown
	ctx, cancel := context.WithTimeout(context.Background(), ShutdownTimeout)
	defer cancel()

	s.logger.Info("shutting down server")
	if err := s.app.ShutdownWithContext(ctx); err != nil {
		return fmt.Errorf("shutdown error: %w", err)
	}

	s.logger.Info("server stopped gracefully")
	return nil
}
