#!/bin/bash
set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Workspace root is one level up
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "Building DBus Notification LED Blinker"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust/Cargo is not installed. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check if dbus-monitor is installed
if ! command -v dbus-monitor &> /dev/null; then
    echo "Error: dbus-monitor is not installed. This is required for the application to work."
    echo "Install dbus package for your distribution:"
    echo "  Fedora/RHEL: sudo dnf install dbus"
    echo "  Debian/Ubuntu: sudo apt install dbus"
    echo "  Arch: sudo pacman -S dbus"
    exit 1
fi

# Build the project in release mode
echo "Building release binary..."
cd "$WORKSPACE_DIR"
cargo build --release --package dbus-notification-led-blinker

# Check if binary was created
if [ ! -f "target/release/dbus-notification-led-blinker" ]; then
    echo "Error: Build failed - binary not found"
    exit 1
fi

echo "Build successful!"

# Explain what the installation will do
echo ""
echo "Installation will perform the following actions:"
echo "  1. Install sudoers file to /etc/sudoers.d/dbus-notification-led-blinker (requires sudo)"
echo "     - Allows passwordless control of Caps Lock LED for user '$USER'"
echo "  2. Copy binary to ~/.local/bin/dbus-notification-led-blinker"
echo "  3. Install systemd service to ~/.config/systemd/user/"
echo "  4. Enable and start the service"
echo ""

# Check if user wants to install
read -p "Install systemd service? [y/N]: " -n 1 -r
echo

if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Get filter patterns
    echo ""
    echo "Enter notification filter patterns (regex). Examples:"
    echo "  - 'urgent' - match notifications containing 'urgent'"
    echo "  - 'error|critical' - match 'error' or 'critical'"
    echo "  - 'Signal' - match notifications from Signal app"
    echo ""
    echo "Enter filter pattern:"
    read -r FILTER_PATTERN
    
    if [ -z "$FILTER_PATTERN" ]; then
        echo "Error: Filter pattern is required"
        exit 1
    fi
    
    # Ask for case sensitivity
    read -p "Use case-insensitive matching? [Y/n]: " -n 1 -r
    echo
    CASE_FLAG=""
    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        CASE_FLAG=" --case-insensitive"
    fi
    
    # Stop existing service if running
    echo "Stopping existing service..."
    systemctl --user stop dbus-notification-led-blinker.service 2>/dev/null || true
    systemctl --user disable dbus-notification-led-blinker.service 2>/dev/null || true
    
    # Create directories if they don't exist
    mkdir -p "$HOME/.local/bin"
    mkdir -p "$HOME/.config/systemd/user"
    
    # Install sudoers file
    echo "Installing sudoers configuration..."
    SUDOERS_TEMP="/tmp/dbus-notification-led-blinker-sudoers"
    sed "s|ALL ALL=(ALL)|$USER ALL=(ALL)|" "$SCRIPT_DIR/sudoers-dbus-notification-led-blinker" > "$SUDOERS_TEMP"
    
    if sudo cp "$SUDOERS_TEMP" /etc/sudoers.d/dbus-notification-led-blinker && sudo chmod 440 /etc/sudoers.d/dbus-notification-led-blinker; then
        echo "Sudoers configuration installed to /etc/sudoers.d/dbus-notification-led-blinker"
        rm "$SUDOERS_TEMP"
    else
        echo "Warning: Failed to install sudoers configuration"
        echo "You may need to manually run:"
        echo "  sudo cp $SCRIPT_DIR/sudoers-dbus-notification-led-blinker /etc/sudoers.d/dbus-notificationdbus-notification-led-blinker"
        echo "  sudo chmod 440 /etc/sudoers.d/dbus-notification-led-blinker"
        rm "$SUDOERS_TEMP"
    fi
    
    # Copy binary to standard location
    BINARY_PATH="$HOME/.local/bin/dbus-notificationdbus-notification-led-blinker"
    echo "Installing binary to $BINARY_PATH..."
    cp "$WORKSPACE_DIR/target/release/dbus-notificationdbus-notification-led-blinker" "$BINARY_PATH"
    chmod +x "$BINARY_PATH"
    
    # Copy and update service file
    SERVICE_PATH="$HOME/.config/systemd/user/dbus-notificationdbus-notification-led-blinker.service"
    echo "Installing service file to $SERVICE_PATH..."
    
    # Update service file with user's filter pattern
    sed "s|--filter \"YOUR_FILTER_PATTERN\"|--filter \"$FILTER_PATTERN\"$CASE_FLAG|" \
        "$SCRIPT_DIR/dbus-notificationdbus-notification-led-blinker.service" > "$SERVICE_PATH"
    
    # Reload systemd and start service
    echo "Reloading systemd and starting service..."
    systemctl --user daemon-reload
    systemctl --user enable dbus-notificationdbus-notification-led-blinker.service
    systemctl --user start dbus-notificationdbus-notification-led-blinker.service
    
    echo ""
    echo "Service installed and started!"
    echo "Binary installed to: $BINARY_PATH"
    echo "Service file installed to: $SERVICE_PATH"
    echo "Filter pattern: $FILTER_PATTERN"
    echo ""
    echo "Check status with: systemctl --user status dbus-notificationdbus-notification-led-blinker.service"
    echo "View logs with: journalctl --user -u dbus-notificationdbus-notification-led-blinker.service -f"
    echo ""
    echo "To add more filters, edit the service file and add multiple --filter options:"
    echo "  --filter \"pattern1\" --filter \"pattern2\""
else
    echo "You can run manually with:"
    echo "   $WORKSPACE_DIR/target/release/dbus-notificationdbus-notification-led-blinker --filter \"YOUR_PATTERN\""
fi
