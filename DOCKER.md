# Docker Deployment Guide

## Quick Start

Pull and run the latest image from GitHub Container Registry:

```bash
docker pull ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest
docker run -d -p 3000:3000 -v ./data:/app/data ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest
```

Access the app at `http://localhost:3000`

## Using Docker Compose

1. Download the docker-compose.yml:
```bash
curl -O https://raw.githubusercontent.com/YOUR_GITHUB_USERNAME/flowbridge/main/web/docker-compose.yml
```

2. Update the image URL in `docker-compose.yml` with your GitHub username

3. Run:
```bash
docker-compose up -d
```

4. Access at `http://localhost:3001`

## For Private Repositories

If your repository is private, authenticate first:

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_GITHUB_USERNAME --password-stdin
```

Then pull/run as normal.

## Environment Variables

- `RUST_LOG`: Set logging level (default: `info`, options: `debug`, `warn`, `error`)
- `DATA_DIR`: Data storage directory (default: `/app/data`)

## Volumes

- `/app/data`: Persistent storage for tasks and configuration

## Ports

- `3000`: Web application port

## Building Locally

To build the image yourself:

```bash
cd web
docker build -t flowbridge:local .
docker run -d -p 3000:3000 -v ./data:/app/data flowbridge:local
```

## Automatic Builds

This repository uses GitHub Actions to automatically build and publish Docker images:

- **On push to main**: Tagged as `latest` and with commit SHA
- **On version tags** (e.g., `v1.0.0`): Tagged with version numbers
- **On pull requests**: Built but not pushed

## Deployment Options

### 1. Self-hosted (VPS/Cloud)

```bash
# On your server
docker pull ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest
docker run -d -p 80:3000 --restart unless-stopped \
  -v /opt/flowbridge/data:/app/data \
  ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest
```

### 2. Railway.app

1. Create new project
2. Deploy from GitHub repo
3. Railway auto-detects Dockerfile
4. Set root directory to `web/`

### 3. Fly.io

```bash
cd web
fly launch
fly deploy
```

### 4. DigitalOcean App Platform

1. Create new app from GitHub
2. Point to `web/` directory
3. Select Dockerfile deployment
4. Deploy

### 5. AWS ECS/Azure Container Instances/Google Cloud Run

Use the public image URL: `ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest`

## Making the Image Public

To allow anyone to pull without authentication:

1. Go to your GitHub repository
2. Click on "Packages" → your image
3. Click "Package settings"
4. Under "Danger Zone", change visibility to "Public"

Now anyone can pull with:
```bash
docker pull ghcr.io/YOUR_GITHUB_USERNAME/flowbridge:latest
```
