# swwws Configuration Guide

This guide covers all configuration options for swwws, the slideshow daemon for swww.

## Table of Contents

- [Configuration File Location](#configuration-file-location)
- [Configuration Structure](#configuration-structure)
- [Global Settings](#global-settings)
- [Monitor Behavior](#monitor-behavior)
- [Per-Output Configuration](#per-output-configuration)
- [Example Configurations](#example-configurations)
- [Additional Configuration](#additional-configuration)
- [Troubleshooting](#troubleshooting)

## Configuration File Location

swwws looks for its configuration file in the following location:
```
~/.config/swwws/config.toml
```

If the file doesn't exist, swwws will create a default configuration on first run.

## Configuration Structure

The configuration file uses TOML format and has three main sections:

```toml
# Global settings applied to all outputs
[global]
# Global configuration options

# Monitor behavior configuration
monitor_behavior = "Independent"  # or "Synchronized" or "Grouped"

# Default settings for any output not specifically configured
[any]
# Default path and settings

# Per-output specific settings
[outputs."OUTPUT-NAME"]
# Output-specific configuration
```

## Global Settings

### Basic Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `duration` | Duration String | `"3m"` | How long to display each image |
| `sorting` | String | `"Random"` | Image sorting method |
| `transition_type` | String | `"center"` | swww transition effect |
| `resize` | String | `"crop"` | Image resize method |
| `fill_color` | String | `"000000"` | Fill color for padding (hex) |
| `filter` | String | `"Lanczos3"` | Image scaling filter |
| `invert_y` | Boolean | `false` | Invert Y position for transitions |
| `queue_size` | Integer | `1000` | Number of images to queue ahead |

### Duration Format

Duration can be specified in human-readable format:
- `"30s"` - 30 seconds
- `"5m"` - 5 minutes  
- `"1h30m"` - 1 hour 30 minutes
- `"2h"` - 2 hours

### Sorting Options

| Option | Description |
|--------|-------------|
| `"Random"` | Shuffle images randomly |
| `"Ascending"` | Sort alphabetically A-Z |
| `"Descending"` | Sort alphabetically Z-A |

### Queue Cycling Behavior

swwws implements queue cycling for continuous slideshow operation:

- Infinite Cycling: Queues automatically restart from the beginning when all images are displayed
- Sort Preservation: Cycling respects the original sorting method (Random reshuffles, Ascending/Descending maintain order)
- State Awareness: CLI commands (`swwws-cli next/previous`) work through queue boundaries
- No Manual Intervention: Slideshow never gets stuck and continuously cycles through your image collection

**Example**: With 10 images and Random sorting:
1. Images 1-10 display in random order
2. Queue automatically restarts with fresh random shuffle
3. Process repeats indefinitely

### Transition Types

All swww transition effects are supported:

| Type | Description |
|------|-------------|
| `"simple"` | Simple fade transition |
| `"fade"` | Smooth fade with bezier curves |
| `"center"` | Expand from center outward |
| `"outer"` | Contract from edges inward |
| `"wipe"` | Wipe across the screen |
| `"left"` | Wipe from left to right |
| `"right"` | Wipe from right to left |
| `"top"` | Wipe from top to bottom |
| `"bottom"` | Wipe from bottom to top |
| `"any"` | Random center/outer transition |
| `"random"` | Completely random transition |

### Image Scaling Modes (resize)

| Mode | Description |
|------|-------------|
| `"crop"` | Resize to fill screen, cropping parts that don't fit (default) |
| `"fit"` | Resize to fit inside screen, preserving aspect ratio |
| `"stretch"` | Resize to fit screen, ignoring aspect ratio |
| `"no"` | Don't resize, center image and pad with fill_color |

### Image Scaling Filters (filter)

| Filter | Description | Best For |
|--------|-------------|----------|
| `"Lanczos3"` | High-quality scaling (default) | Most images |
| `"Nearest"` | Pixel-perfect scaling | Pixel art only |
| `"Bilinear"` | Fast, good quality | General use |
| `"CatmullRom"` | Sharp, detailed scaling | Detailed images |
| `"Mitchell"` | Balanced quality/performance | General use |

### Fill Colors (fill_color)

Specify as 6-digit hex color codes without the `#` prefix:
- `"000000"` - Black (default)
- `"FFFFFF"` - White  
- `"FF0000"` - Red
- `"808080"` - Gray

### Transition Settings

```toml
[global]
transition_step = 90              # Transition smoothness (1-255)
transition_angle = 90.0           # Angle for wipe/wave transitions (degrees)
transition_pos = "center"         # Transition starting position
transition_duration = "500ms"     # How long transitions take
transition_fps = 30               # Transition frame rate
```

**Transition Angle Details:**
- **Used with**: `wipe` and `wave` transition types
- **Format**: Floating-point degrees (e.g., `45.0`, `90.0`, `180.0`)
- **Direction**: `0.0` = right to left, `90.0` = top to bottom, `270.0` = bottom to top
- **Range**: Any degree value (wraps around at 360)
- **Examples**:
  - `45.0` - Diagonal wipe from top-right to bottom-left
  - `135.0` - Diagonal wipe from top-left to bottom-right
  - `180.0` - Left to right wipe

**Transition Wave (transition_wave):**
- **Used with**: `wave` transition type only
- **Format**: `"width,height"` (e.g., `"20,20"`)
- **Controls**: Width and height of wave pattern
- **Default**: `"20,20"`
- **Examples**:
  - `"10,10"` - Smaller, tighter waves
  - `"40,20"` - Wide, short waves
  - `"20,40"` - Narrow, tall waves

**Invert Y (invert_y):**
- **Used with**: `grow` and `outer` transitions with `transition_pos`
- **Format**: Boolean (`true`/`false`)
- **Effect**: Inverts the Y coordinate of transition position
- **Default**: `false`

### File Discovery Settings

```toml
[global]
recursive = true                  # Search subdirectories recursively
image_path = "/path/to/default"   # Fallback image path (deprecated, use [any])
```

## Monitor Behavior

swwws supports three different monitor behaviors:

### Independent Mode (Default)
Each monitor operates independently with its own queue and timing.

```toml
monitor_behavior = "Independent"

[outputs."HDMI-A-1"]
path = "/path/to/work/wallpapers"
duration = "10m"

[outputs."DP-2"]  
path = "/path/to/gaming/wallpapers"
duration = "3m"
```

### Synchronized Mode
All monitors display the same image at the same time.

```toml
monitor_behavior = "Synchronized"

[any]
path = "/path/to/shared/wallpapers"
duration = "5m"
```

### Grouped Mode
Custom groups of monitors that sync together, with different groups operating independently.

```toml
monitor_behavior = "Grouped"

# Define monitor groups
monitor_groups = [
    ["HDMI-A-1", "DP-2"],    # Group 1: Main workspace
    ["DP-3"]                 # Group 2: Secondary display
]

# Configuration applies to groups, not individual monitors
[any]
path = "/path/to/wallpapers"
duration = "5m"
```

## Per-Output Configuration

### Output Names

Find your output names using:
```bash
swww query
```

Example output names:
- `HDMI-A-1` - HDMI port 1
- `DP-2` - DisplayPort 2
- `eDP-1` - Embedded DisplayPort (laptop screen)

### Output-Specific Settings

```toml
[outputs."HDMI-A-1"]
path = "/path/to/work/wallpapers"
duration = "15m"
sorting = "Ascending"
transition_type = "fade"
transition_step = 200
mode = "fit"

[outputs."DP-2"]
path = "/path/to/personal/wallpapers"
duration = "3m"
sorting = "Random"
transition_type = "wipe"
mode = "fill"
```

### The `[any]` Section

The `[any]` section provides defaults for outputs not explicitly configured:

```toml
[any]
path = "/path/to/default/wallpapers"
duration = "5m"
sorting = "Random"
```

## Example Configurations

### Minimal Configuration

```toml
monitor_behavior = "Synchronized"

[any]
path = "/home/user/Pictures/Wallpapers"
```

### Gaming Setup (Multiple Monitors)

```toml
monitor_behavior = "Independent"

[global]
transition_type = "fade"
transition_duration = "200ms"

[outputs."HDMI-A-1"]  # Main gaming monitor
path = "/home/user/Pictures/Gaming"
duration = "2m"
sorting = "Random"
mode = "fill"

[outputs."DP-2"]      # Secondary monitor
path = "/home/user/Pictures/Minimal"  
duration = "10m"
sorting = "Ascending"
mode = "fit"

[outputs."DP-3"]      # Vertical monitor
path = "/home/user/Pictures/Vertical"
duration = "5m"
mode = "center"
```

### Work Setup (Grouped Monitors)

```toml
monitor_behavior = "Grouped"

# Main work displays sync together, vertical display independent
monitor_groups = [
    ["HDMI-A-1", "DP-2"],    # Main workspace
    ["DP-3"]                 # Vertical display
]

[global]
transition_type = "simple"
transition_step = 255        # Instant transitions
queue_size = 5

[any]
path = "/home/user/Pictures/Professional"
duration = "30m"
sorting = "Ascending"
mode = "fit"
```

### Slideshow Setup (Art Display)

```toml
monitor_behavior = "Synchronized"

[global]
duration = "1m"              # Change every minute
sorting = "Random"
transition_type = "fade"
transition_duration = "2s"   # Slow, smooth transitions
transition_step = 50         # Very smooth
queue_size = 20              # Large queue for variety

[any]
path = "/home/user/Pictures/Art"
resize = "fit"               # Preserve aspect ratio
recursive = true             # Include all subdirectories
```

### Transitions Setup

```toml
monitor_behavior = "Independent"

[global]
transition_type = "wipe"
transition_angle = 45.0      # Diagonal wipe
transition_step = 120
resize = "crop"
fill_color = "1a1a1a"        # Dark gray padding
filter = "Lanczos3"          # High-quality scaling

[outputs."HDMI-A-1"]         # Gaming monitor
path = "/home/user/Pictures/Gaming"
transition_type = "wave"
transition_wave = "30,15"    # Wide, short waves
transition_angle = 90.0      # Top to bottom
resize = "crop"              # Fill entire screen

[outputs."DP-2"]             # Vertical monitor  
path = "/home/user/Pictures/Vertical"
transition_type = "grow"
transition_pos = "center"
resize = "fit"               # Preserve aspect ratio
fill_color = "000000"        # Black padding
invert_y = false

[outputs."DP-3"]             # Art display
path = "/home/user/Pictures/Art"
transition_type = "outer"
transition_pos = "top-left"
resize = "no"                # No scaling, original size
fill_color = "FFFFFF"        # White background
filter = "Nearest"           # Pixel-perfect for art
invert_y = true              # Invert Y position
```

## Additional Configuration

### Multiple Image Directories

To use multiple directories, create symbolic links or use a script to organize your images:

```bash
# Create a combined directory with symlinks
mkdir -p ~/.local/share/swwws/combined
ln -s /path/to/nature/* ~/.local/share/swwws/combined/
ln -s /path/to/abstract/* ~/.local/share/swwws/combined/
```

Then configure:
```toml
[any]
path = "/home/user/.local/share/swwws/combined"
```

### Hot Reloading Configuration

swwws supports hot reloading of configuration:

```bash
# Reload configuration without restarting daemon
swwws-cli reload
```

**Features:**
- Monitor Behavior Changes: Automatically detects changes between Independent/Synchronized/Grouped modes
- State Preservation: Maintains queue positions and wallpapers when possible
- No Restart Required: No daemon restart required, even for major behavior changes
- Instant Application: Changes take effect immediately

**What gets reloaded:**
- Global settings (durations, transitions, etc.)
- Monitor behavior mode switching
- Per-output configurations
- Queue and timing settings

**Example workflow:**
```bash
# Edit config to change from Independent to Synchronized
vim ~/.config/swwws/config.toml

# Apply changes instantly
swwws-cli reload
# ✓ Configuration reloaded and daemon state reinitialized for new monitor behavior
```

### State Persistence

swwws maintains state in:
```
~/.local/state/swwws/state.json
```

To reset state (clear queue positions, etc.):
```bash
systemctl --user stop swwws
rm -f ~/.local/state/swwws/state.json
systemctl --user start swwws
```

### Logging Configuration

Control logging via environment variables:

```bash
# Set log level (error, warn, info, debug, trace)
export RUST_LOG=swwws_daemon=info

# Start daemon with logging
swwws-daemon
```

Or in systemd service:
```ini
[Service]
Environment=RUST_LOG=info
```

## Supported Image Formats

swwws supports all image formats that swww supports:
- **JPEG** (.jpg, .jpeg)
- **PNG** (.png)
- **GIF** (.gif) - including animated
- **BMP** (.bmp)
- **TIFF** (.tiff, .tif)
- **WebP** (.webp) - including animated
- **AVIF** (.avif)
- **SVG** (.svg) - static only

## Troubleshooting

### Common Issues

**Images not changing**: 
- Check that swww-daemon is running: `pgrep swww-daemon`
- Verify image path exists and contains supported images
- Try manually advancing: `swwws-cli next`
- Check logs: `journalctl --user -u swwws.service -f`

**Wrong monitor names**:
- Get correct names: `swww query`
- Monitor names are case-sensitive
- Names may change after system reboot

**Configuration not loading**:
- Check TOML syntax: Use an online TOML validator
- Verify file location: `~/.config/swwws/config.toml`
- Check file permissions: `ls -la ~/.config/swwws/config.toml`

**Monitor behavior not switching**:
- Use hot reload: `swwws-cli reload` 
- Check configuration syntax: Use an online TOML validator
- Verify behavior change detected in logs: `journalctl --user -u swwws.service -f`
- If issues persist, restart daemon: `systemctl --user restart swwws`

### Debug Mode

Run daemon in debug mode for detailed logging:

```bash
# Stop service
systemctl --user stop swwws

# Run in foreground with debug logging
RUST_LOG=debug swwws-daemon

# In another terminal, test commands
swwws-cli status
swwws-cli next
```

### Configuration Validation

Validate your configuration:

```bash
# Test configuration reload
swwws-cli reload

# Check daemon status
swwws-cli status

# View current configuration effect
swww query
```

## Getting Help

- **CLI Help**: `swwws-cli --help`
- **Daemon Help**: `swwws-daemon --help`
- **Example Configs**: Check `examples/configs/` directory
- **Logs**: `journalctl --user -u swwws.service -f`

For additional help, check the main [README](README.md) or open an issue on GitHub.
