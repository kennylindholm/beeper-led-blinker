#!/bin/bash

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Workspace root is one level up
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building Beeper LED Blinker"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build the project in release mode
echo "Building release binary..."
cd "$WORKSPACE_DIR"
cargo build --release

# Check if binary was created
if [ ! -f "target/release/beeper-led-blinker" ]; then
    echo "Error: Build failed - binary not found"
    exit 1
fi

echo "Build successful!"

# Explain what the installation will do
echo ""
echo "Installation will perform the following actions:"
echo "  1. Install sudoers file to /etc/sudoers.d/beeper-blinker (requires sudo)"
echo "     - Allows passwordless control of Caps Lock LED for user '$USER'"
echo "  2. Copy binary to ~/.local/bin/beeper-led-blinker"
echo "  3. Install systemd service to ~/.config/systemd/user/"
echo "  4. Enable and start the service"
echo ""

# Check if user wants to install
read -p "Install systemd service? [y/N]: " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Get token from environment variable or prompt
    if [ -z "$BEEPER_API_TOKEN" ]; then
        echo ""
        echo "Enter your Beeper API token:"
        echo "(You can also set BEEPER_API_TOKEN environment variable to skip this prompt)"
        read -r BEEPER_API_TOKEN

        if [ -z "$BEEPER_API_TOKEN" ]; then
            echo "Error: Token is required"
            exit 1
        fi
    else
        echo "Using token from BEEPER_API_TOKEN environment variable"
    fi

    # Stop existing service if running
    echo "Stopping existing service..."
    systemctl --user stop beeper-led-blinker.service 2>/dev/null || true
    systemctl --user disable beeper-led-blinker.service 2>/dev/null || true

    # Create directories if they don't exist
    mkdir -p "$HOME/.local/bin"
    mkdir -p "$HOME/.config/systemd/user"

    # Install sudoers file
    echo "Installing sudoers configuration..."
    SUDOERS_TEMP="/tmp/beeper-led-blinker-sudoers"
    sed "s|ALL ALL=(ALL)|$USER ALL=(ALL)|" "$SCRIPT_DIR/sudoers-beeper-led-blinker" > "$SUDOERS_TEMP"

    if sudo cp "$SUDOERS_TEMP" /etc/sudoers.d/beeper-led-blinker && sudo chmod 440 /etc/sudoers.d/beeper-led-blinker; then
        echo "Sudoers configuration installed to /etc/sudoers.d/beeper-led-blinker"
        rm "$SUDOERS_TEMP"
    else
        echo "Warning: Failed to install sudoers configuration"
        echo "You may need to manually run:"
        echo "  sudo cp sudoers-beeper-led-blinker /etc/sudoers.d/beeper-led-blinker"
        echo "  sudo chmod 440 /etc/sudoers.d/beeper-led-blinker"
    fi

    # Copy binary to standard location
    BINARY_PATH="$HOME/.local/bin/beeper-led-blinker"
    echo "Installing binary to $BINARY_PATH..."
    cp "$WORKSPACE_DIR/target/release/beeper-led-blinker" "$BINARY_PATH"
    chmod +x "$BINARY_PATH"

    # Copy and update service file
    SERVICE_PATH="$HOME/.config/systemd/user/beeper-led-blinker.service"
    echo "Installing service file to $SERVICE_PATH..."
    sed "s|--token YOUR_TOKEN|--token $BEEPER_API_TOKEN|" \
        "$SCRIPT_DIR/beeper-led-blinker.service" > "$SERVICE_PATH"

    # Reload systemd and start service
    echo "Reloading systemd and starting service..."
    systemctl --user daemon-reload
    systemctl --user enable beeper-led-blinker.service
    systemctl --user start beeper-led-blinker.service

    echo "Service installed and started!"
    echo "Binary installed to: $BINARY_PATH"
    echo "Service file installed to: $SERVICE_PATH"
    echo ""
    echo "Check status with: systemctl --user status beeper-led-blinker.service"
    echo "View logs with: journalctl --user -u beeper-led-blinker.service -f"
else
    echo "You can run manually with:"
    echo "   $WORKSPACE_DIR/target/release/beeper-led-blinker --token YOUR_TOKEN"
fi
