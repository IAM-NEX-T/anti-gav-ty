package gateway

import (
	"context"
	"fmt"
	"time"

	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"go.uber.org/zap"
)

// Gateway represents a registered agent.
type Gateway struct {
	ID         string    `json:"id"`
	Name       string    `json:"name"`
	MacAddress string    `json:"mac_address"`
	IPAddress  string    `json:"ip_address"`
	Hostname   string    `json:"hostname"`
	Version    string    `json:"version"`
	Status     string    `json:"status"`
	LastSeen   time.Time `json:"last_seen"`
	CreatedAt  time.Time `json:"created_at"`
	UpdatedAt  time.Time `json:"updated_at"`
}

// Repository handles gateway database operations.
type Repository struct {
	pool   *pgxpool.Pool
	logger *zap.Logger
}

// NewRepository creates a new gateway repository.
func NewRepository(pool *pgxpool.Pool, logger *zap.Logger) *Repository {
	return &Repository{
		pool:   pool,
		logger: logger,
	}
}

// Register creates or updates a gateway record.
func (r *Repository) Register(ctx context.Context, gw *Gateway) error {
	query := `
		INSERT INTO gateways (name, mac_address, ip_address, hostname, version, status, last_seen)
		VALUES ($1, $2, $3, $4, $5, 'online', NOW())
		ON CONFLICT (mac_address)
		DO UPDATE SET
			name = EXCLUDED.name,
			ip_address = EXCLUDED.ip_address,
			hostname = EXCLUDED.hostname,
			version = EXCLUDED.version,
			status = 'online',
			last_seen = NOW(),
			updated_at = NOW()
		RETURNING id, created_at, updated_at
	`

	err := r.pool.QueryRow(ctx, query,
		gw.Name,
		gw.MacAddress,
		gw.IPAddress,
		gw.Hostname,
		gw.Version,
	).Scan(&gw.ID, &gw.CreatedAt, &gw.UpdatedAt)

	if err != nil {
		return fmt.Errorf("registering gateway: %w", err)
	}

	gw.Status = "online"
	gw.LastSeen = time.Now()

	r.logger.Info("gateway registered",
		zap.String("id", gw.ID),
		zap.String("name", gw.Name),
		zap.String("mac", gw.MacAddress),
	)

	return nil
}

// GetByMacAddress retrieves a gateway by its MAC address.
func (r *Repository) GetByMacAddress(ctx context.Context, mac string) (*Gateway, error) {
	query := `
		SELECT id, name, mac_address, ip_address, hostname, version, status, last_seen, created_at, updated_at
		FROM gateways
		WHERE mac_address = $1
	`

	gw := &Gateway{}
	var lastSeen, createdAt, updatedAt time.Time

	err := r.pool.QueryRow(ctx, query, mac).Scan(
		&gw.ID,
		&gw.Name,
		&gw.MacAddress,
		&gw.IPAddress,
		&gw.Hostname,
		&gw.Version,
		&gw.Status,
		&lastSeen,
		&createdAt,
		&updatedAt,
	)
	if err != nil {
		if err == pgx.ErrNoRows {
			return nil, nil
		}
		return nil, fmt.Errorf("getting gateway: %w", err)
	}

	gw.LastSeen = lastSeen
	gw.CreatedAt = createdAt
	gw.UpdatedAt = updatedAt

	return gw, nil
}

// List returns all registered gateways.
func (r *Repository) List(ctx context.Context) ([]Gateway, error) {
	query := `
		SELECT id, name, mac_address, ip_address, hostname, version, status, last_seen, created_at, updated_at
		FROM gateways
		ORDER BY created_at DESC
	`

	rows, err := r.pool.Query(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("listing gateways: %w", err)
	}
	defer rows.Close()

	var gateways []Gateway
	for rows.Next() {
		var gw Gateway
		var lastSeen, createdAt, updatedAt time.Time

		err := rows.Scan(
			&gw.ID,
			&gw.Name,
			&gw.MacAddress,
			&gw.IPAddress,
			&gw.Hostname,
			&gw.Version,
			&gw.Status,
			&lastSeen,
			&createdAt,
			&updatedAt,
		)
		if err != nil {
			return nil, fmt.Errorf("scanning gateway: %w", err)
		}

		gw.LastSeen = lastSeen
		gw.CreatedAt = createdAt
		gw.UpdatedAt = updatedAt

		gateways = append(gateways, gw)
	}

	if gateways == nil {
		gateways = []Gateway{}
	}

	return gateways, nil
}

// UpdateStatus updates a gateway's online/offline status.
func (r *Repository) UpdateStatus(ctx context.Context, mac string, status string) error {
	query := `
		UPDATE gateways
		SET status = $2, last_seen = NOW(), updated_at = NOW()
		WHERE mac_address = $1
	`

	_, err := r.pool.Exec(ctx, query, mac, status)
	if err != nil {
		return fmt.Errorf("updating gateway status: %w", err)
	}

	return nil
}
