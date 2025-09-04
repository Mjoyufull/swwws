use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Result, Context};
use crate::queue::Sorting;

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputState {
    pub current_image: Option<String>,
    pub queue_position: usize,
    pub queue_size: usize,
    pub sorting: Sorting,
    pub images: Vec<String>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonState {
    pub outputs: HashMap<String, OutputState>,
    pub global_paused: bool,
    pub last_save: chrono::DateTime<chrono::Utc>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            outputs: HashMap::new(),
            global_paused: false,
            last_save: chrono::Utc::now(),
        }
    }

    pub fn save(&self, state_file: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = state_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create state directory: {:?}", parent))?;
        }

        let json = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize state to JSON")?;
        
        fs::write(state_file, json)
            .with_context(|| format!("Failed to write state file: {:?}", state_file))?;
        
        log::debug!("State saved to {:?}", state_file);
        Ok(())
    }

    pub fn load(state_file: &Path) -> Result<Self> {
        if !state_file.exists() {
            log::info!("No state file found, starting fresh");
            return Ok(Self::new());
        }

        let json = fs::read_to_string(state_file)
            .with_context(|| format!("Failed to read state file: {:?}", state_file))?;
        
        let state: Self = serde_json::from_str(&json)
            .with_context(|| "Failed to deserialize state from JSON")?;
        
        log::info!("State loaded from {:?}", state_file);
        Ok(state)
    }

    pub fn get_state_file() -> PathBuf {
        dirs::state_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("swwws")
            .join("state.json")
    }

    pub fn update_output_state(
        &mut self,
        output_name: &str,
        current_image: Option<&Path>,
        queue_position: usize,
        queue_size: usize,
        sorting: Sorting,
        images: &[PathBuf],
    ) {
        let output_state = OutputState {
            current_image: current_image.map(|p| p.to_string_lossy().to_string()),
            queue_position,
            queue_size,
            sorting,
            images: images.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            last_updated: chrono::Utc::now(),
        };
        
        self.outputs.insert(output_name.to_string(), output_state);
        self.last_save = chrono::Utc::now();
    }

    pub fn get_output_state(&self, output_name: &str) -> Option<&OutputState> {
        self.outputs.get(output_name)
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.global_paused = paused;
        self.last_save = chrono::Utc::now();
    }

    pub fn is_paused(&self) -> bool {
        self.global_paused
    }

    pub fn is_stale(&self, max_age_hours: u64) -> bool {
        let now = chrono::Utc::now();
        let age = now - self.last_save;
        age.num_hours() > max_age_hours as i64
    }

    pub fn cleanup_stale_state(&mut self, max_age_hours: u64) {
        let now = chrono::Utc::now();
        self.outputs.retain(|_, output_state| {
            let age = now - output_state.last_updated;
            age.num_hours() <= max_age_hours as i64
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::path::PathBuf;

    #[test]
    fn test_state_save_load() {
        let temp_dir = tempdir().unwrap();
        let state_file = temp_dir.path().join("test_state.json");
        
        let mut state = DaemonState::new();
        state.update_output_state(
            "test-output",
            Some(Path::new("/test/image.jpg")),
            5,
            10,
            Sorting::Random,
            &[PathBuf::from("/test/image1.jpg"), PathBuf::from("/test/image2.jpg")],
        );
        
        // Save state
        state.save(&state_file).unwrap();
        
        // Load state
        let loaded_state = DaemonState::load(&state_file).unwrap();
        
        // Verify state was preserved
        let output_state = loaded_state.get_output_state("test-output").unwrap();
        assert_eq!(output_state.current_image, Some("/test/image.jpg".to_string()));
        assert_eq!(output_state.queue_position, 5);
        assert_eq!(output_state.queue_size, 10);
        assert_eq!(output_state.images.len(), 2);
    }

    #[test]
    fn test_stale_state_cleanup() {
        let mut state = DaemonState::new();
        
        // Add some outputs
        state.update_output_state(
            "recent-output",
            Some(Path::new("/test/recent.jpg")),
            1,
            10,
            Sorting::Random,
            &[PathBuf::from("/test/recent.jpg")],
        );
        
        // Simulate old state by setting last_updated to 25 hours ago
        if let Some(output_state) = state.outputs.get_mut("recent-output") {
            output_state.last_updated = chrono::Utc::now() - chrono::Duration::hours(25);
        }
        
        // Cleanup stale state (older than 24 hours)
        state.cleanup_stale_state(24);
        
        // Should be cleaned up
        assert!(state.get_output_state("recent-output").is_none());
    }
}
