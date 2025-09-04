use std::path::Path;
use std::process::Command;
use crate::error::{SwwwsError, ProcessError};
use crate::Result;
use crate::command_builder::{CommandBuilder, OutputConfig};

#[derive(Clone)]
pub struct ProcessExecutor {
    command_builder: CommandBuilder,
}

impl ProcessExecutor {
    pub fn new(command_builder: CommandBuilder) -> Self {
        Self { command_builder }
    }

    pub async fn execute_swww_command(
        &self,
        image_path: &Path,
        config: &OutputConfig,
        output_name: Option<&str>,
    ) -> Result<()> {
        // Validate the image path first
        crate::image_discovery::ImageDiscovery::validate_image(image_path)?;

        let mut command = self.command_builder.build_img_command(
            image_path,
            config,
            output_name,
        );

        log::info!("Executing swww command: {:?}", command);

        let output = command.output()
            .map_err(|e| SwwwsError::Process(ProcessError::Execution {
                command: format!("{:?}", command),
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
        let output = Command::new("swww")
            .arg("query")
            .output()
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
        const MAX_RETRIES: u32 = 10;
        const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(500);
        
        for attempt in 0..MAX_RETRIES {
            let output = Command::new("swww")
                .arg("query")
                .output()
                .map_err(|e| SwwwsError::Process(ProcessError::Execution {
                    command: "swww query".to_string(),
                    source: e,
                }))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
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

            if !outputs.is_empty() {
                log::info!("Found swww outputs: {:?}", outputs);
                return Ok(outputs);
            }
            
            if attempt < MAX_RETRIES - 1 {
                log::info!("No outputs found yet (attempt {}/{}), retrying in {}ms...", 
                    attempt + 1, MAX_RETRIES, RETRY_DELAY.as_millis());
                std::thread::sleep(RETRY_DELAY);
            }
        }
        
        log::error!("No swww outputs found after {} attempts. Possible issues:", MAX_RETRIES);
        log::error!("- swww-daemon may still be initializing");
        log::error!("- No monitors detected by wayland compositor");
        log::error!("- swww-daemon may have crashed during initialization");
        
        Err(SwwwsError::Process(ProcessError::NonZeroExit {
            code: -1,
            stderr: "No swww outputs found after retries. Check swww-daemon status and monitor connections.".to_string(),
        }))
    }
}
