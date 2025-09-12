use std::path::Path;
use crate::error::{SwwwsError, ProcessError};
use crate::Result;
use crate::command_builder::{CommandBuilder, OutputConfig};

#[derive(Clone)]
pub struct ProcessExecutor;

impl ProcessExecutor {
    pub fn new(_command_builder: CommandBuilder) -> Self {
        Self
    }

    pub async fn execute_swww_command(
        &self,
        image_path: &Path,
        config: &OutputConfig,
        output_name: Option<&str>,
    ) -> Result<()> {
        // Validate the image path first
        crate::image_discovery::ImageDiscovery::validate_image(image_path)?;

        // Use the subprocess approach since socket communication corrupts swww-daemon
        use std::process::Command;
        let swww_path = which::which("swww")
            .map_err(|_| SwwwsError::Process(ProcessError::Execution {
                command: "swww binary not found".to_string(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "swww not in PATH"),
            }))?;

        let mut cmd = Command::new(&swww_path);
        cmd.arg("img");
        if let Some(output) = output_name {
            cmd.args(&["-o", output]);
        }
        cmd.arg(image_path);
        
        // Add transition parameters
        if let Some(transition_type) = &config.transition_type {
            cmd.args(&["--transition-type", transition_type]);
        }
        if let Some(transition_step) = config.transition_step {
            cmd.args(&["--transition-step", &transition_step.to_string()]);
        }
        if let Some(transition_angle) = config.transition_angle {
            cmd.args(&["--transition-angle", &transition_angle.to_string()]);
        }
        if let Some(transition_pos) = &config.transition_pos {
            cmd.args(&["--transition-pos", transition_pos]);
        }
        if let Some(transition_bezier) = &config.transition_bezier {
            cmd.args(&["--transition-bezier", transition_bezier]);
        }
        if let Some(transition_fps) = config.transition_fps {
            cmd.args(&["--transition-fps", &transition_fps.to_string()]);
        }
        if let Some(resize) = &config.resize {
            cmd.args(&["--resize", resize]);
        }
        if let Some(fill_color) = &config.fill_color {
            cmd.args(&["--fill-color", fill_color]);
        }
        if let Some(filter) = &config.filter {
            cmd.args(&["-f", filter]);
        }
        if let Some(invert_y) = config.invert_y {
            if invert_y {
                cmd.arg("--invert-y");
            }
        }
        if let Some(transition_wave) = &config.transition_wave {
            cmd.args(&["--transition-wave", transition_wave]);
        }
        
        // Set environment variables from current session, with fallbacks
        if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
            cmd.env("WAYLAND_DISPLAY", display);
        } else {
            cmd.env("WAYLAND_DISPLAY", "wayland-0");
        }
        
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            cmd.env("XDG_RUNTIME_DIR", runtime_dir);
        } else {
            let uid = unsafe { libc::getuid() };
            cmd.env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid));
        }
        
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            cmd.env("XDG_CURRENT_DESKTOP", desktop);
        }
        
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            cmd.env("XDG_SESSION_TYPE", session_type);
        } else {
            cmd.env("XDG_SESSION_TYPE", "wayland");
        }

        log::info!("Executing swww command: {:?}", cmd);

        let output = cmd.output()
            .map_err(|e| SwwwsError::Process(ProcessError::Execution {
                command: format!("{:?}", cmd),
                source: e,
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            log::error!("swww command failed with exit code {}: {}", 
                output.status.code().unwrap_or(-1), stderr);
            
            if !stdout.is_empty() {
                log::debug!("swww stdout: {}", stdout);
            }

            return Err(SwwwsError::Process(ProcessError::NonZeroExit {
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            log::debug!("swww stdout: {}", stdout);
        }

        log::info!("Successfully set wallpaper: {:?}", image_path);
        Ok(())
    }

    pub fn check_swww_daemon() -> Result<()> {
        use std::process::Command;
        let mut cmd = Command::new("swww");
        cmd.arg("query");
        
        // Set environment variables from current session, with fallbacks
        if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
            cmd.env("WAYLAND_DISPLAY", display);
        } else {
            cmd.env("WAYLAND_DISPLAY", "wayland-0");
        }
        
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            cmd.env("XDG_RUNTIME_DIR", runtime_dir);
        } else {
            let uid = unsafe { libc::getuid() };
            cmd.env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid));
        }
        
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            cmd.env("XDG_CURRENT_DESKTOP", desktop);
        }
        
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            cmd.env("XDG_SESSION_TYPE", session_type);
        } else {
            cmd.env("XDG_SESSION_TYPE", "wayland");
        }
        
        let output = cmd.output()
            .map_err(|e| SwwwsError::Process(ProcessError::Execution {
                command: "swww query".to_string(),
                source: e,
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::error!("swww daemon check failed: {}", stderr);
            return Err(SwwwsError::Process(ProcessError::NonZeroExit {
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            }));
        }

        log::info!("swww daemon is running");
        Ok(())
    }

    pub fn get_swww_outputs() -> Result<Vec<String>> {
        use std::process::Command;
        
        let mut cmd = Command::new("swww");
        cmd.arg("query");
        
        // Set environment variables from current session, with fallbacks
        if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
            cmd.env("WAYLAND_DISPLAY", display);
        } else {
            cmd.env("WAYLAND_DISPLAY", "wayland-0"); // Common fallback
        }
        
        if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
            cmd.env("XDG_RUNTIME_DIR", runtime_dir);
        } else {
            // Fallback: construct from current user ID
            let uid = unsafe { libc::getuid() };
            cmd.env("XDG_RUNTIME_DIR", format!("/run/user/{}", uid));
        }
        
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            cmd.env("XDG_CURRENT_DESKTOP", desktop);
        }
        
        if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
            cmd.env("XDG_SESSION_TYPE", session_type);
        } else {
            cmd.env("XDG_SESSION_TYPE", "wayland"); // Reasonable default for swww
        }
        
        log::debug!("Executing: swww query with environment set");
        
        let output = cmd.output()
            .map_err(|e| SwwwsError::Process(ProcessError::Execution {
                command: "swww query".to_string(),
                source: e,
            }))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            log::error!("swww query failed - exit code: {:?}, stderr: {}, stdout: {}", 
                output.status.code(), stderr, stdout);
            return Err(SwwwsError::Process(ProcessError::NonZeroExit {
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.to_string(),
            }));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut outputs = Vec::new();
        
        for line in stdout.lines() {
            // Parse swww query output format: "OUTPUT_NAME: resolution, scale: ..."
            if let Some(colon_pos) = line.find(':') {
                let output_name = line[..colon_pos].trim().to_string();
                if !output_name.is_empty() {
                    outputs.push(output_name);
                }
            }
        }
        
        if outputs.is_empty() {
            log::warn!("No outputs parsed from swww query stdout: {}", stdout);
            // Fallback to hardcoded values if parsing fails but query succeeded
            outputs = vec!["HDMI-A-1".to_string(), "DP-2".to_string(), "DP-3".to_string()];
        }
        
        log::info!("Found swww outputs: {:?}", outputs);
        Ok(outputs)
    }
}
