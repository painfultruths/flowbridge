#!/bin/bash
set -e

echo "🚀 Starting Task Manager - Modern Edition..."
echo ""

# Build if needed
if [ ! -f "target/release/task-web" ]; then
    echo "Building application..."
    cargo build --release
fi

# Run the server
echo "Server starting at http://localhost:3000"
echo "Press Ctrl+C to stop"
echo ""
./target/release/task-web
