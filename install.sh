#!/bin/bash

# swwws Install Script
# This script installs swwws to the system

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="$HOME/.config/swwws"
SERVICE_DIR="$HOME/.config/systemd/user"

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to detect privilege escalation command
detect_privesc() {
    if command_exists doas; then
        echo "doas"
    elif command_exists sudo; then
        echo "sudo"
    else
        echo "none"
    fi
}

# Function to run command with privilege escalation
run_as_root() {
    # If already root, no need for privilege escalation
    if [ "$(id -u)" -eq 0 ]; then
        "$@"
        return $?
    fi
    
    local privesc=$(detect_privesc)
    case $privesc in
        "doas")
            print_status "Using doas for privilege escalation..."
            doas "$@"
            ;;
        "sudo")
            print_status "Using sudo for privilege escalation..."
            sudo "$@"
            ;;
        "none")
            print_error "Neither sudo nor doas found. Cannot install system-wide binaries."
            print_status "You can build locally and copy binaries manually to ~/bin/"
            exit 1
            ;;
    esac
}

# Function to check if systemd is available
has_systemd() {
    # Check if systemctl exists and if we're in a systemd environment
    command_exists systemctl && [ -d "/run/systemd/system" ]
}

# Function to check if swwws is already installed
check_existing_installation() {
    local found_installations=()
    
    # Check for swwws binaries
    if command_exists swwws-cli; then
        found_installations+=("swwws-cli binary in PATH")
    fi
    if command_exists swwws; then
        found_installations+=("swwws (alias) binary in PATH")
    fi
    
    if [ -f "$INSTALL_DIR/swwws" ]; then
        found_installations+=("swwws (alias) binary in $INSTALL_DIR")
    fi
    if [ -f "$INSTALL_DIR/swwws-cli" ]; then
        found_installations+=("swwws-cli binary in $INSTALL_DIR")
    fi
    
    if [ -f "$INSTALL_DIR/swwws-daemon" ]; then
        found_installations+=("swwws-daemon binary in $INSTALL_DIR")
    fi
    
    # Check for configuration
    if [ -d "$CONFIG_DIR" ]; then
        found_installations+=("configuration in $CONFIG_DIR")
    fi
    
    # Check for systemd service
    if [ -f "$SERVICE_DIR/swwws.service" ]; then
        found_installations+=("systemd service in $SERVICE_DIR")
    fi
    
    if [ ${#found_installations[@]} -gt 0 ]; then
        print_warning "Found existing swwws installation:"
        for item in "${found_installations[@]}"; do
            echo "  - $item"
        done
        
        echo
        read -p "Do you want to remove the existing installation? (y/N): " -n 1 -r
        echo
        
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_status "Removing existing installation..."
            
            # Remove binaries
            if command_exists swwws; then
                run_as_root rm -f "$(which swwws)"
            fi
            if command_exists swwws-cli; then
                run_as_root rm -f "$(which swwws-cli)"
            fi
            
            if [ -f "$INSTALL_DIR/swwws" ]; then
                run_as_root rm -f "$INSTALL_DIR/swwws"
            fi
            if [ -f "$INSTALL_DIR/swwws-cli" ]; then
                run_as_root rm -f "$INSTALL_DIR/swwws-cli"
            fi
            
            if [ -f "$INSTALL_DIR/swwws-daemon" ]; then
                run_as_root rm -f "$INSTALL_DIR/swwws-daemon"
            fi
            
            # Remove configuration
            if [ -d "$CONFIG_DIR" ]; then
                rm -rf "$CONFIG_DIR"
            fi
            
            # Remove systemd service (if systemd is available)
            if has_systemd && [ -f "$SERVICE_DIR/swwws.service" ]; then
                rm -f "$SERVICE_DIR/swwws.service"
                systemctl --user daemon-reload
            fi
            
            print_success "Existing installation removed"
        else
            print_error "Installation cancelled. Please remove existing installation manually."
            exit 1
        fi
    fi
}

# Function to check dependencies
check_dependencies() {
    print_status "Checking dependencies..."
    
    local missing_deps=()
    
    # Check for Rust
    if ! command_exists cargo; then
        missing_deps+=("rustc and cargo")
    fi
    
    # Check for swww
    if ! command_exists swww; then
        missing_deps+=("swww")
    fi
    
    # Check for swww-daemon
    if ! command_exists swww-daemon; then
        missing_deps+=("swww-daemon")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing dependencies:"
        for dep in "${missing_deps[@]}"; do
            echo "  - $dep"
        done
        
        echo
        print_status "Please install the missing dependencies:"
        echo "  - Rust: https://rustup.rs/"
        echo "  - swww: https://github.com/LGFae/swww"
        exit 1
    fi
    
    print_success "All dependencies found"
}

# Function to build swwws
build_swwws() {
    print_status "Building swwws..."
    
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the swwws directory."
        exit 1
    fi
    
    # Build in release mode
    cargo build --release
    
    if [ $? -ne 0 ]; then
        print_error "Build failed"
        exit 1
    fi
    
    print_success "Build completed"
}

# Function to install binaries
install_binaries() {
    print_status "Installing binaries to $INSTALL_DIR..."
    
    # Copy the binaries to install directory
    run_as_root cp "$(pwd)/target/release/swwws-daemon" "$INSTALL_DIR/swwws-daemon"
    # Install CLI under the canonical name 'swwws-cli'
    run_as_root cp "$(pwd)/target/release/swwws-cli" "$INSTALL_DIR/swwws-cli"
    # Backward-compat alias 'swwws' for existing docs/scripts (symlink is fine here since both files are permanent)
    run_as_root ln -sf "$INSTALL_DIR/swwws-cli" "$INSTALL_DIR/swwws"
    
    # Make sure they're executable
    run_as_root chmod +x "$INSTALL_DIR/swwws-daemon"
    run_as_root chmod +x "$INSTALL_DIR/swwws-cli"
    run_as_root chmod +x "$INSTALL_DIR/swwws"
    
    print_success "Binaries installed (swwws-daemon, swwws-cli, alias: swwws)"
}

# Function to create configuration
create_configuration() {
    print_status "Creating configuration directory..."
    
    mkdir -p "$CONFIG_DIR"
    
    # Create default configuration if it doesn't exist
    if [ ! -f "$CONFIG_DIR/config.toml" ]; then
        cat > "$CONFIG_DIR/config.toml" << 'EOF'
# swwws Configuration File

# Global settings applied to all outputs unless overridden
[global]
duration = "3m"
sorting = "Random"
transition_type = "center"
transition_step = 90
queue_size = 1000
recursive = true

# Monitor behavior: Independent, Synchronized, or Grouped
monitor_behavior = "Independent"

# Shared path for all outputs
[any]
path = "/path/to/wallpaper"

# Per-output specific configuration (optional)
# [outputs."HDMI-A-1"]
# path = "/different/path/for/this/monitor"
# duration = "5m"
# transition_type = "wipe"
EOF
        print_success "Default configuration created at $CONFIG_DIR/config.toml"
        print_warning "Please edit the configuration file to set your wallpaper paths"
    else
        print_status "Configuration already exists at $CONFIG_DIR/config.toml"
    fi
}

# Function to create systemd service
create_systemd_service() {
    if ! has_systemd; then
        print_warning "systemd not detected - skipping service creation"
        print_status "You can manually start swwws-daemon or use your init system"
        return 0
    fi
    
    print_status "Creating systemd service..."
    
    mkdir -p "$SERVICE_DIR"
    
    cat > "$SERVICE_DIR/swwws.service" << EOF
[Unit]
Description=swwws slideshow daemon
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/swwws-daemon
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=graphical-session.target
EOF
    
    # Reload systemd
    systemctl --user daemon-reload
    
    print_success "Systemd service created"
    print_status "To enable the service, run: systemctl --user enable swwws"
    print_status "To start the service, run: systemctl --user start swwws"
}

# Function to verify installation
verify_installation() {
    print_status "Verifying installation..."
    
    local errors=0
    
    # Check if binaries are accessible
    if ! command_exists swwws-cli; then
        print_error "swwws-cli command not found in PATH"
        errors=$((errors + 1))
    fi
    
    if ! command_exists swwws-daemon; then
        print_error "swwws-daemon command not found in PATH"
        errors=$((errors + 1))
    fi
    
    # Check if configuration exists
    if [ ! -f "$CONFIG_DIR/config.toml" ]; then
        print_error "Configuration file not found"
        errors=$((errors + 1))
    fi
    
    # Check if systemd service exists (only if systemd is available)
    if has_systemd && [ ! -f "$SERVICE_DIR/swwws.service" ]; then
        print_error "Systemd service not found"
        errors=$((errors + 1))
    fi
    
    if [ $errors -eq 0 ]; then
        print_success "Installation verified successfully"
        echo
        print_status "Installation complete! Next steps:"
        echo "  1. Edit configuration: $CONFIG_DIR/config.toml"
        if has_systemd; then
            echo "  2. Enable service: systemctl --user enable swwws"
            echo "  3. Start service: systemctl --user start swwws"
            echo "  4. Control slideshow: swwws-cli next, swwws-cli previous, etc. (alias: swwws)"
        else
            echo "  2. Start daemon manually: swwws-daemon &"
            echo "  3. Control slideshow: swwws-cli next, swwws-cli previous, etc. (alias: swwws)"
            echo "  4. Set up auto-start with your init system (OpenRC, runit, etc.)"
        fi
    else
        print_error "Installation verification failed with $errors errors"
        exit 1
    fi
}

# Main installation function
main() {
    echo "=========================================="
    echo "           swwws Install Script"
    echo "=========================================="
    echo
    
    # Check for existing installation (skip if called from quick-install)
    if [ "$SWWWS_SKIP_EXISTING_CHECK" != "1" ]; then
        check_existing_installation
    fi
    
    # Check dependencies
    check_dependencies
    
    # Build swwws
    build_swwws
    
    # Install binaries
    install_binaries
    
    # Create configuration
    create_configuration
    
    # Create systemd service
    create_systemd_service
    
    # Verify installation
    verify_installation
    
    echo
    echo "=========================================="
    print_success "Installation completed successfully!"
    echo "=========================================="
}

# Run main function
main "$@"
