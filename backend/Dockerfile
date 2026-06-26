# Stage 1: Build
FROM golang:1.22-alpine AS builder

RUN apk add --no-cache ca-certificates tzdata

WORKDIR /build

# Cache dependencies
COPY go.mod go.sum ./
RUN go mod download

# Build
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo \
    -ldflags="-w -s" \
    -o /build/server \
    ./cmd/server

# Stage 2: Final
FROM scratch

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=builder /build/server /server
COPY configs/backend.yaml /configs/backend.yaml

USER 1000:1000

EXPOSE 8080

ENTRYPOINT ["/server"]
