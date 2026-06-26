package gateway

import (
	"time"

	"github.com/gofiber/fiber/v2"
	"go.uber.org/zap"
)

// Handler handles gateway HTTP requests.
type Handler struct {
	repo   *Repository
	logger *zap.Logger
}

// NewHandler creates a new gateway handler.
func NewHandler(repo *Repository, logger *zap.Logger) *Handler {
	return &Handler{
		repo:   repo,
		logger: logger,
	}
}

// RegisterRequest is the payload sent by the agent on registration.
type RegisterRequest struct {
	Name       string `json:"name"`
	MacAddress string `json:"mac_address"`
	IPAddress  string `json:"ip_address"`
	Hostname   string `json:"hostname"`
	Version    string `json:"version"`
}

// RegisterResponse is returned after successful registration.
type RegisterResponse struct {
	Gateway Gateway `json:"gateway"`
}

// HeartbeatRequest is sent by the agent periodically.
type HeartbeatRequest struct {
	MacAddress string `json:"mac_address"`
	Version    string `json:"version"`
}

// HeartbeatResponse is returned after a successful heartbeat.
type HeartbeatResponse struct {
	Status string `json:"status"`
}

// HandleRegister processes gateway registration.
func (h *Handler) HandleRegister(c *fiber.Ctx) error {
	var req RegisterRequest
	if err := c.BodyParser(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
			"error": "invalid request body",
		})
	}

	if req.MacAddress == "" {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
			"error": "mac_address is required",
		})
	}

	gw := &Gateway{
		Name:       req.Name,
		MacAddress: req.MacAddress,
		IPAddress:  req.IPAddress,
		Hostname:   req.Hostname,
		Version:    req.Version,
		Status:     "online",
		LastSeen:   time.Now(),
	}

	if err := h.repo.Register(c.Context(), gw); err != nil {
		h.logger.Error("failed to register gateway", zap.Error(err))
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
			"error": "failed to register gateway",
		})
	}

	return c.Status(fiber.StatusOK).JSON(RegisterResponse{
		Gateway: *gw,
	})
}

// HandleHeartbeat processes agent heartbeat pings.
func (h *Handler) HandleHeartbeat(c *fiber.Ctx) error {
	var req HeartbeatRequest
	if err := c.BodyParser(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
			"error": "invalid request body",
		})
	}

	if req.MacAddress == "" {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{
			"error": "mac_address is required",
		})
	}

	// Update last_seen and status
	if err := h.repo.Heartbeat(c.Context(), req.MacAddress, req.Version); err != nil {
		h.logger.Error("failed to process heartbeat", zap.Error(err))
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
			"error": "failed to process heartbeat",
		})
	}

	return c.JSON(HeartbeatResponse{
		Status: "ok",
	})
}

// HandleList returns all registered gateways.
func (h *Handler) HandleList(c *fiber.Ctx) error {
	gateways, err := h.repo.List(c.Context())
	if err != nil {
		h.logger.Error("failed to list gateways", zap.Error(err))
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
			"error": "failed to list gateways",
		})
	}

	return c.JSON(fiber.Map{
		"gateways": gateways,
	})
}

// HandleGetByMac returns a specific gateway by MAC address.
func (h *Handler) HandleGetByMac(c *fiber.Ctx) error {
	mac := c.Params("mac")

	gw, err := h.repo.GetByMacAddress(c.Context(), mac)
	if err != nil {
		h.logger.Error("failed to get gateway", zap.Error(err))
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{
			"error": "failed to get gateway",
		})
	}

	if gw == nil {
		return c.Status(fiber.StatusNotFound).JSON(fiber.Map{
			"error": "gateway not found",
		})
	}

	return c.JSON(fiber.Map{
		"gateway": gw,
	})
}

// RegisterRoutes adds gateway routes to the router.
func (h *Handler) RegisterRoutes(router fiber.Router) {
	router.Post("/gateways", h.HandleRegister)
	router.Get("/gateways", h.HandleList)
	router.Get("/gateways/:mac", h.HandleGetByMac)
	router.Post("/gateways/heartbeat", h.HandleHeartbeat)
}
