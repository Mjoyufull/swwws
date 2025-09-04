#!/bin/bash

# swwws Quick Install Script (Curl-friendly)
# This script is designed to be run via curl and will handle everything

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
REPO_URL="https://github.com/Mjoyufull/swwws.git"
# Use unique temporary directory to avoid conflicts with existing projects
TEMP_SUFFIX="$(date +%s)_$$_$(shuf -i 1000-9999 -n 1 2>/dev/null || echo $RANDOM)"
CLONE_DIR="/tmp/swwws_install_${TEMP_SUFFIX}"
INSTALL_DIR="/usr/local/bin"

# Cleanup function to remove temporary directory
cleanup() {
    if [ -d "$CLONE_DIR" ]; then
        print_status "Cleaning up temporary directory: $CLONE_DIR"
        rm -rf "$CLONE_DIR"
    fi
}

# Set trap to cleanup on exit (including errors)
trap cleanup EXIT INT TERM

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
            print_error "Neither sudo nor doas found. Cannot remove system-wide binaries."
            print_status "You may need to manually remove binaries from $INSTALL_DIR"
            ;;
    esac
}

# Function to check if systemd is available
has_systemd() {
    # Check if systemctl exists and if we're in a systemd environment
    command_exists systemctl && [ -d "/run/systemd/system" ]
}

# Function to check dependencies
check_dependencies() {
    print_status "Checking dependencies..."
    
    local missing_deps=()
    
    # Check for Git
    if ! command_exists git; then
        missing_deps+=("git")
    fi
    
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
        echo "  - Git: Use your system package manager (apt, dnf, pacman, etc.)"
        echo "  - Rust: https://rustup.rs/"
        echo "  - swww: https://github.com/LGFae/swww"
        echo
        print_status "After installing dependencies, run this command again:"
        echo "  curl -fsSL https://raw.githubusercontent.com/Mjoyufull/swwws/main/quick-install.sh | bash"
        exit 1
    fi
    
    print_success "All dependencies found"
}

# Function to clone repository to temporary directory
setup_repository() {
    print_status "Cloning swwws repository to temporary directory..."
    
    # Clone repository to unique temporary directory
    print_status "Cloning repository to $CLONE_DIR"
    git clone "$REPO_URL" "$CLONE_DIR"
    
    cd "$CLONE_DIR"
    print_success "Repository cloned successfully"
}

# Function to check for existing installation and handle it
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
    
    # Check for configuration (but don't remove it - user might want to keep settings)
    if [ -d "$HOME/.config/swwws" ]; then
        found_installations+=("configuration in $HOME/.config/swwws (will be preserved)")
    fi
    
    # Check for systemd service
    if [ -f "$HOME/.config/systemd/user/swwws.service" ]; then
        found_installations+=("systemd service in $HOME/.config/systemd/user")
    fi
    
    if [ ${#found_installations[@]} -gt 0 ]; then
        print_warning "Found existing swwws installation:"
        for item in "${found_installations[@]}"; do
            echo "  - $item"
        done
        
        echo
        print_status "Removing existing installation automatically..."
        
        # Remove binaries
        if command_exists swwws; then
            run_as_root rm -f "$(which swwws)" 2>/dev/null || true
        fi
        if command_exists swwws-cli; then
            run_as_root rm -f "$(which swwws-cli)" 2>/dev/null || true
        fi
        if command_exists swwws-daemon; then
            run_as_root rm -f "$(which swwws-daemon)" 2>/dev/null || true
        fi
        
        run_as_root rm -f "$INSTALL_DIR/swwws" 2>/dev/null || true
        run_as_root rm -f "$INSTALL_DIR/swwws-cli" 2>/dev/null || true
        run_as_root rm -f "$INSTALL_DIR/swwws-daemon" 2>/dev/null || true
        
        # Stop service if running (only if systemd is available)
        if has_systemd; then
            systemctl --user stop swwws 2>/dev/null || true
            systemctl --user disable swwws 2>/dev/null || true
            
            # Remove systemd service
            if [ -f "$HOME/.config/systemd/user/swwws.service" ]; then
                rm -f "$HOME/.config/systemd/user/swwws.service"
                systemctl --user daemon-reload
            fi
        fi
        
        print_success "Existing installation removed"
    fi
}

# Function to run the main install script
run_install() {
    print_status "Running swwws installation..."
    
    if [ ! -f "install.sh" ]; then
        print_error "install.sh not found in repository"
        exit 1
    fi
    
    # Make sure install script is executable
    chmod +x install.sh
    
    # Run the install script (skipping existing installation check since we handled it)
    SWWWS_SKIP_EXISTING_CHECK=1 ./install.sh
}

# Function to show post-install information
show_completion() {
    echo
    echo "=========================================="
    print_success "swwws installation completed!"
    echo "=========================================="
    echo
    print_status "Binary location: $INSTALL_DIR"
    echo
    print_status "Next steps:"
    echo "  1. Edit configuration: ~/.config/swwws/config.toml"
    if has_systemd; then
        echo "  2. Enable service: systemctl --user enable swwws"
        echo "  3. Start service: systemctl --user start swwws"
        echo "  4. Control slideshow: swwws-cli next, swwws-cli previous, etc."
    else
        echo "  2. Start daemon manually: swwws-daemon &"
        echo "  3. Control slideshow: swwws-cli next, swwws-cli previous, etc."
        echo "  4. Set up auto-start with your init system (OpenRC, runit, etc.)"
    fi
    echo
    print_status "Documentation and examples:"
    echo "  - Online: https://github.com/Mjoyufull/swwws"
    echo "  - Configuration guide: https://github.com/Mjoyufull/swwws/blob/main/CONFIGURATION.md"
    echo "  - Example configs: https://github.com/Mjoyufull/swwws/tree/main/examples/configs"
    echo
    print_status "For help: swwws-cli --help"
}

# Main installation function
main() {
    echo "=========================================="
    echo "        swwws Quick Install Script"
    echo "=========================================="
    echo
    print_status "This script will:"
    echo "  - Check system dependencies"
    echo "  - Clone swwws repository to a temporary directory"
    echo "  - Build and install swwws"
    echo "  - Set up configuration and systemd service"
    echo "  - Clean up temporary files"
    echo
    
    # Check dependencies first
    check_dependencies
    
    # Check for existing installation before cloning
    check_existing_installation
    
    # Set up repository
    setup_repository
    
    # Run the main install script
    run_install
    
    # Show completion information
    show_completion
}

# Run main function
main "$@"
