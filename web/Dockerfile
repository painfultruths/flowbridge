# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install CA certificates for HTTPS
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/task-web /app/task-web

# Copy static files
COPY static ./static

# Create data directory
RUN mkdir -p /app/data

# Set data directory environment variable
ENV DATA_DIR=/app/data

# Expose port
EXPOSE 3000

# Create volume for persistent data
VOLUME ["/app/data"]

# Run the binary
CMD ["./task-web"]
