use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, Context};

pub struct SwwwIntegration {
    swww_path: PathBuf,
}

impl SwwwIntegration {
    pub fn new() -> Result<Self> {
        let swww_path = which::which("swww")
            .with_context(|| "swww not found in PATH")?;
        Ok(Self { swww_path })
    }
    
    pub fn check_daemon_running(&self) -> Result<bool> {
        let output = Command::new(&self.swww_path)
            .arg("query")
            .output()
            .with_context(|| "Failed to execute swww query")?;
        
        Ok(output.status.success())
    }
    
    pub fn get_available_outputs(&self) -> Result<Vec<String>> {
        let output = Command::new(&self.swww_path)
            .arg("query")
            .output()
            .with_context(|| "Failed to execute swww query")?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("swww query failed"));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut outputs = Vec::new();
        
        for line in output_str.lines() {
            // Parse lines like ": HDMI-A-1: 1920x1080, scale: 1, currently displaying: image: ..."
            if line.starts_with(": ") {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let output_name = parts[1].trim();
                    if !output_name.is_empty() {
                        outputs.push(output_name.to_string());
                    }
                }
            }
        }
        
        Ok(outputs)
    }
    
    pub fn get_swww_path(&self) -> PathBuf {
        self.swww_path.clone()
    }
}
