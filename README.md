# swwws - Slideshow Daemon for swww

**swwws** is a slideshow daemon that extends [swww](https://github.com/LGFae/swww) with automated wallpaper cycling and multi-monitor support. It combines swww's smooth transitions with slideshow functionality.
<img width="1920" height="1080" alt="1757017957_grim" src="https://github.com/user-attachments/assets/a8dc7540-3425-49f1-96be-d6c0fa34e620" />
<img width="1920" height="1080" alt="1757017929_grim" src="https://github.com/user-attachments/assets/c1acf6a7-8bec-4db8-9094-f3d6f7cfa33c" />



## Features

- Automatic wallpaper cycling with human-readable timers (3m, 1h30m, etc.)
- Queue management with infinite cycling and sorting options
- Multi-monitor support with synchronized, independent, or grouped behaviors
- Runtime control via CLI (next, previous, pause, resume, status)
- Hot configuration reload with monitor behavior switching
- State persistence - remembers position and settings across restarts
- All swww transition effects with per-monitor customization
- Error handling with retry logic and daemon recovery

## Architecture

```
swwws-daemon (scheduler/queue manager)
    ↓ constructs and executes commands
swww img <path> [transition-flags...]
    ↓ handles display & transitions
swww-daemon → wayland compositor
```

## Features

### Core Functionality
- Image Discovery: Recursive directory scanning for supported formats
- Queue Management: Queue system with infinite cycling and history tracking
- Timer System: Human-readable duration parsing ("3m", "1h", "30s")
- Multi-monitor: Support for different monitor behaviors with seamless switching
- IPC Interface: Unix socket communication for CLI control

### Configuration
- TOML-based: Easy-to-edit configuration files
- Inheritance: Global and per-output settings with inheritance
- Hot-reload: Configuration changes without daemon restart
- Validation: Configuration validation
<img width="841" height="604" alt="Screenshot_20250904-203434" src="https://github.com/user-attachments/assets/f839a124-e5ef-4b1e-82be-9b2b1454e1fd" />


### CLI Commands
- `swwws-cli next` - Advance to next wallpaper (infinite cycling)
- `swwws-cli previous` - Go to previous wallpaper
- `swwws-cli pause/resume/toggle-pause` - Control slideshow
- `swwws-cli reload` - Hot reload configuration (including monitor behavior changes)
- `swwws-cli status` - Show current state and queue information
<img width="999" height="787" alt="Screenshot_20250904-203405" src="https://github.com/user-attachments/assets/6d58d037-e387-4b7a-891b-e9d0ae4b69d8" />



## Installation

### Quick Install (Recommended)

Install swwws directly with a single command:
```bash
curl -fsSL https://raw.githubusercontent.com/Mjoyufull/swwws/main/quick-install.sh | bash
```

**Prerequisites**: Ensure you have [Rust](https://rustup.rs/) and [swww](https://github.com/LGFae/swww) installed first.

### Alternative Installation Methods

#### Manual Installation with Git

```bash
git clone https://github.com/Mjoyufull/swwws.git
cd swwws
./install.sh
```

#### Build from Source Only

```bash
git clone https://github.com/Mjoyufull/swwws.git
cd swwws
cargo build --release
cp target/release/{swwws-daemon,swwws-cli} ~/.local/bin/
```

### What the Installer Does

The installation scripts will:
- Build swwws in release mode with optimizations
- Install binaries to `/usr/local/bin` (with sudo) or `~/.local/bin`
- Create configuration directory at `~/.config/swwws/`
- Set up systemd user service for automatic startup
- Provide example configurations

## Quick Start

### Prerequisites
- [swww](https://github.com/LGFae/swww) installed and running
- Rust toolchain (1.87.0+)
- Wayland compositor with wlr-layer-shell support

### Basic Setup

1. **Start swww daemon**:
   ```bash
   swww-daemon
   ```

2. **Create configuration** (`~/.config/swwws/config.toml`):
   ```toml
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
   
   # Per-output overrides (optional)
   # [outputs."DP-1"]
   # path = "/different/path/for/this/monitor"
   # duration = "5m"
   # transition_type = "wipe"
   ```

3. **Start swwws daemon**:
   ```bash
   swwws-daemon
   ```

4. **Control slideshow**:
   ```bash
   swwws-cli next          # Next wallpaper
   swwws-cli previous      # Previous wallpaper
   swwws-cli pause         # Pause slideshow
   swwws-cli resume        # Resume slideshow
   swwws-cli status        # Show status
   swwws-cli reload        # Reload configuration
   ```

## Service Management

### Systemd Service (Recommended)

The installer creates a systemd user service for automatic startup:

```bash
# Enable automatic startup
systemctl --user enable swwws

# Start the service
systemctl --user start swwws

# Check service status
systemctl --user status swwws

# View logs
journalctl --user -u swwws.service -f

# Restart service (after config changes)
systemctl --user restart swwws
```

### Manual Daemon Management

Alternatively, run the daemon manually:

```bash
# Start daemon in background
swwws-daemon &

# Or start in foreground with logs
swwws-daemon

# Stop daemon
pkill swwws-daemon
```

## Configuration

swwws uses a TOML configuration file located at `~/.config/swwws/config.toml`.

### Quick Configuration Examples

**Simple setup (all monitors, same wallpapers):**
```toml
[global]
duration = "3m"
sorting = "Random"
transition_type = "center"
queue_size = 1000
monitor_behavior = "Independent"

[any]
path = "/home/user/Pictures/Wallpapers"
```

**Multi-monitor with different wallpapers:**
```toml
[global]
duration = "3m"
sorting = "Random"
transition_type = "center"
queue_size = 1000
monitor_behavior = "Independent"

[outputs."HDMI-A-1"]
path = "/home/user/Pictures/Work"
duration = "15m"

[outputs."DP-2"]
path = "/home/user/Pictures/Personal"
duration = "3m"
```

**Synchronized setup (all monitors show same image):**
```toml
[global]
duration = "5m"
sorting = "Random"
transition_type = "center"
monitor_behavior = "Synchronized"

[any]
path = "/home/user/Pictures/Wallpapers"
```

### Complete Configuration Guide

For complete configuration documentation, see **[CONFIGURATION.md](CONFIGURATION.md)** which covers:

- **All configuration options** with detailed explanations
- **Monitor behavior modes** (Independent, Synchronized, Grouped)
- **Transition settings** (transitions, timing, image discovery)
- **Real-world examples** (gaming setups, work environments, art displays)
- **Troubleshooting guide** for common configuration issues

### Example Configurations

Ready-to-use configuration examples are available in [`examples/configs/`](examples/configs/):
- [`synchronized.toml`](examples/configs/synchronized.toml) - All monitors show same image
- [`independent.toml`](examples/configs/independent.toml) - Each monitor independent
- [`grouped.toml`](examples/configs/grouped.toml) - Custom monitor groupings

Copy an example to get started:
```bash
cp examples/configs/synchronized.toml ~/.config/swwws/config.toml
```

## Supported Image Formats

swwws supports all image formats that swww supports:
- JPEG, PNG, GIF, BMP, TIFF, WebP, AVIF
- Animated GIFs
- SVG (static only)

## Transition Effects

All swww transition effects are supported:
- `simple` - Simple fade
- `center` - Center to edges
- `outer` - Edges to center
- `wipe` - Wipe transition
- `left`, `right`, `top`, `bottom` - Directional wipes
- `any` - Random center/outer
- `random` - Random transition type

## Multi-Monitor Support

swwws provides three monitor behavior modes:

### Independent Mode (Default)
- Each monitor operates independently with its own wallpaper queue
- Different content per display - work wallpapers on main monitor, nature on secondary
- Individual timing - main monitor changes every 15 minutes, secondary every 3 minutes
- Suitable for productivity setups where each monitor serves different purposes
- Flexible - allows complete customization per monitor

### Synchronized Mode  
- All monitors display the same image simultaneously
- Single shared queue manages wallpapers for all displays
- Coordinated transitions happen across all monitors at once
- Good for setups where you want consistent wallpapers everywhere

### Grouped Mode
- Custom groups of monitors that sync together within each group
- Different groups operate independently from each other
- Mix of synchronized and independent behavior
- Useful for complex setups like main workspace + secondary displays

**Detailed setup instructions:** See [CONFIGURATION.md](CONFIGURATION.md#monitor-behavior) for complete examples and configuration options.
- Mix synchronized and independent behavior as needed
- Example: Main work displays sync, secondary display independent

## Development

### Project Structure
```
swwws/
├── swwws-daemon/     # Main daemon process
├── swwws-cli/        # Command line interface
├── swwws-common/     # Shared types and utilities
└── swwws-config/     # Configuration parsing
```

### Building
```bash
cargo build --release
```

### Testing
```bash
cargo test
```

## Integration with Existing Tools

### swww Compatibility
swwws is designed to work seamlessly with existing swww setups:
- Uses existing swww daemon
- Preserves all swww functionality
- No changes to swww required

### wpaperd Migration
For users migrating from wpaperd:
- Similar configuration format
- Compatible CLI commands
- Same queue management logic

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your changes
4. Add tests
5. Submit a pull request

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- [swww](https://github.com/LGFae/swww) - The underlying wallpaper daemon
- [wpaperd](https://github.com/danyspin97/wpaperd) - Inspiration for slideshow functionality
- [Smithay](https://github.com/Smithay) - Wayland client toolkit

## Running as a Service

After installation, you can run swwws as a systemd user service:

```bash
# Enable and start the service
systemctl --user enable swwws.service
systemctl --user start swwws.service

# Check status
systemctl --user status swwws.service

# View logs
journalctl --user -u swwws.service -f
```

## Troubleshooting

### Common Issues

**Daemon won't start**: Ensure swww-daemon is running first:
```bash
swww-daemon
```

**No wallpapers changing**: Check the status and logs:
```bash
swwws-cli status
journalctl --user -u swwws.service -f
```

**Configuration errors**: Validate your config file and check for typos in monitor names:
```bash
swww query  # Shows available monitor names
```

## Documentation

- **[CONFIGURATION.md](CONFIGURATION.md)** - Complete configuration guide with examples
- **[examples/configs/](examples/configs/)** - Ready-to-use configuration files
- **[CHANGELOG.md](CHANGELOG.md)** - Version history and changes  
- **CLI Help** - `swwws-cli --help` and `swwws-daemon --help`

## Links

- **[swww](https://github.com/LGFae/swww)** - The underlying wallpaper setter
- **[Rust](https://rustup.rs/)** - Required for building from source
- **Issues & Support** - [GitHub Issues](https://github.com/Mjoyufull/swwws/issues)

## Support

For questions, issues, or contributions:
- **GitHub Issues**: [Report bugs or request features](https://github.com/Mjoyufull/swwws/issues)
- **Documentation**: Check [CONFIGURATION.md](CONFIGURATION.md) for detailed setup
- **Debugging**: Use `RUST_LOG=debug swwws-daemon` for detailed logs
- **Changelog**: Review [CHANGELOG.md](CHANGELOG.md) for recent updates
