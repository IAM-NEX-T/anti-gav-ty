.PHONY: build run test clean docker-build docker-run

# Binary name
BINARY=server

# Build the binary
build:
	go build -o bin/$(BINARY) ./cmd/server

# Run the server locally
run: build
	./bin/$(BINARY) -config configs/backend.yaml

# Run tests
test:
	go test ./... -v -race -count=1

# Run tests with coverage
test-coverage:
	go test ./... -coverprofile=coverage.out
	go tool cover -html=coverage.out -o coverage.html

# Clean build artifacts
clean:
	rm -rf bin/
	rm -f coverage.out coverage.html

# Docker build
docker-build:
	docker build -t anti-gav-ty-backend:latest .

# Docker run
docker-run:
	docker run -p 8080:8080 anti-gav-ty-backend:latest
