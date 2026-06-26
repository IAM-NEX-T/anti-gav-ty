package server

import (
	"net/http/httptest"
	"testing"
	"time"

	"go.uber.org/zap"
)

func TestHealthEndpoint(t *testing.T) {
	logger := zap.NewNop()

	s := New(logger, Config{
		Host:         "localhost",
		Port:         0,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
		IdleTimeout:  60 * time.Second,
	})

	req := httptest.NewRequest("GET", "/api/v1/health", nil)
	resp, err := s.app.Test(req, -1)
	if err != nil {
		t.Fatalf("failed to test health endpoint: %v", err)
	}

	if resp.StatusCode != 200 {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}
}

func TestReadyEndpoint(t *testing.T) {
	logger := zap.NewNop()

	s := New(logger, Config{
		Host:         "localhost",
		Port:         0,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
		IdleTimeout:  60 * time.Second,
	})

	req := httptest.NewRequest("GET", "/api/v1/health/ready", nil)
	resp, err := s.app.Test(req, -1)
	if err != nil {
		t.Fatalf("failed to test ready endpoint: %v", err)
	}

	if resp.StatusCode != 200 {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}
}
