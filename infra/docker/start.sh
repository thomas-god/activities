#!/bin/sh
# start.sh

# Function to handle shutdown
cleanup() {
    echo "Shutting down..."
    kill $API_PID 2>/dev/null
    nginx -s quit
    exit 0
}

# Set up signal handlers
trap cleanup SIGTERM SIGINT

# Start the Rust API in the background
SERVER_PORT=8080 ALLOW_ORIGIN= ./app_bin &
API_PID=$!

# Check if API started successfully
sleep 2
if ! kill -0 $API_PID 2>/dev/null; then
    echo "Failed to start API server"
    exit 1
fi

# Start nginx in the foreground
nginx -g 'daemon off;' &
NGINX_PID=$!

# Wait for any process to exit
wait