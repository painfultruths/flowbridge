#!/bin/bash
# FlowBridge Server Deployment Script

# 1. Install Docker (if not already installed)
# curl -fsSL https://get.docker.com -o get-docker.sh
# sudo sh get-docker.sh

# 2. Login to GitHub Container Registry (only needed if image is private)
# echo $GITHUB_TOKEN | docker login ghcr.io -u painfultruths --password-stdin

# 3. Pull the latest image
docker pull ghcr.io/painfultruths/flowbridge:latest

# 4. Stop and remove old container (if exists)
docker stop flowbridge 2>/dev/null || true
docker rm flowbridge 2>/dev/null || true

# 5. Run the new container
docker run -d \
  --name flowbridge \
  --restart unless-stopped \
  -p 3000:3000 \
  -v /opt/flowbridge/data:/app/data \
  -e RUST_LOG=info \
  ghcr.io/painfultruths/flowbridge:latest

# 6. Check if it's running
docker ps | grep flowbridge

echo "✓ FlowBridge is running on port 3000"
echo "Access it at: http://your-server-ip:3000"
