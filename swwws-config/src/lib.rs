use serde::{Deserialize, Serialize, Deserializer};
use std::path::PathBuf;
#[cfg(test)]
use std::path::Path;
use std::time::Duration;
use swwws_common::{Sorting, MonitorBehavior, SwwwsError, error::ConfigError, Result};

// Custom deserialization for Duration from human-readable strings
fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let duration_str = String::deserialize(deserializer)?;
    swwws_common::duration::parse_duration(&duration_str)
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub any: OutputConfig,
    #[serde(default = "default_monitor_behavior")]
    pub monitor_behavior: MonitorBehavior,
    #[serde(default)]
    pub monitor_groups: Option<Vec<Vec<String>>>,
    #[serde(flatten)]
    pub outputs: std::collections::HashMap<String, OutputConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    #[serde(default = "default_duration", deserialize_with = "deserialize_duration")]
    pub duration: Duration,
    #[serde(default = "default_queue_size")]
    pub queue_size: usize,
    #[serde(default = "default_sorting")]
    pub sorting: Sorting,
    #[serde(default = "default_transition_type")]
    pub transition_type: String,
    #[serde(default = "default_transition_step")]
    pub transition_step: u32,
    #[serde(default = "default_transition_angle")]
    pub transition_angle: f32,
    #[serde(default = "default_transition_pos")]
    pub transition_pos: String,
    #[serde(default = "default_transition_bezier")]
    pub transition_bezier: String,
    #[serde(default = "default_transition_duration", deserialize_with = "deserialize_duration")]
    pub transition_duration: Duration,
    #[serde(default = "default_resize")]
    pub resize: String,
    #[serde(default = "default_fill_color")]
    pub fill_color: String,
    #[serde(default = "default_filter")]
    pub filter: String,
    #[serde(default = "default_invert_y")]
    pub invert_y: bool,
    #[serde(default = "default_transition_wave")]
    pub transition_wave: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OutputConfig {
    pub path: Option<String>,
    #[serde(default = "default_duration", deserialize_with = "deserialize_duration")]
    pub duration: Duration,
    #[serde(default = "default_queue_size")]
    pub queue_size: usize,
    #[serde(default = "default_sorting")]
    pub sorting: Sorting,
    #[serde(default = "default_transition_type")]
    pub transition_type: String,
    #[serde(default = "default_transition_step")]
    pub transition_step: u32,
    #[serde(default = "default_transition_angle")]
    pub transition_angle: f32,
    #[serde(default = "default_transition_pos")]
    pub transition_pos: String,
    #[serde(default = "default_transition_bezier")]
    pub transition_bezier: String,
    #[serde(default = "default_transition_duration", deserialize_with = "deserialize_duration")]
    pub transition_duration: Duration,
    #[serde(default = "default_resize")]
    pub resize: String,
    #[serde(default = "default_fill_color")]
    pub fill_color: String,
    #[serde(default = "default_filter")]
    pub filter: String,
    #[serde(default = "default_invert_y")]
    pub invert_y: bool,
    #[serde(default = "default_transition_wave")]
    pub transition_wave: String,
}

// Default values
fn default_duration() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

fn default_queue_size() -> usize {
    10
}

fn default_sorting() -> Sorting {
    Sorting::Random
}

fn default_transition_type() -> String {
    "wipe".to_string()
}

fn default_transition_step() -> u32 {
    90
}

fn default_transition_angle() -> f32 {
    90.0
}

fn default_transition_pos() -> String {
    "center".to_string()
}

fn default_transition_bezier() -> String {
    "0.25,0.1,0.25,1".to_string()
}

fn default_transition_duration() -> Duration {
    Duration::from_millis(500)
}

fn default_resize() -> String {
    "crop".to_string()
}

fn default_fill_color() -> String {
    "000000".to_string()
}

fn default_filter() -> String {
    "Lanczos3".to_string()
}

fn default_invert_y() -> bool {
    false
}

fn default_transition_wave() -> String {
    "20,20".to_string()
}

fn default_monitor_behavior() -> MonitorBehavior {
    MonitorBehavior::Independent
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            duration: default_duration(),
            queue_size: default_queue_size(),
            sorting: default_sorting(),
            transition_type: default_transition_type(),
            transition_step: default_transition_step(),
            transition_angle: default_transition_angle(),
            transition_pos: default_transition_pos(),
            transition_bezier: default_transition_bezier(),
            transition_duration: default_transition_duration(),
            resize: default_resize(),
            fill_color: default_fill_color(),
            filter: default_filter(),
            invert_y: default_invert_y(),
            transition_wave: default_transition_wave(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            global: GlobalConfig::default(),
            any: OutputConfig::default(),
            monitor_behavior: default_monitor_behavior(),
            monitor_groups: None,
            outputs: std::collections::HashMap::new(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            path: None,
            duration: default_duration(),
            queue_size: default_queue_size(),
            sorting: default_sorting(),
            transition_type: default_transition_type(),
            transition_step: default_transition_step(),
            transition_angle: default_transition_angle(),
            transition_pos: default_transition_pos(),
            transition_bezier: default_transition_bezier(),
            transition_duration: default_transition_duration(),
            resize: default_resize(),
            fill_color: default_fill_color(),
            filter: default_filter(),
            invert_y: default_invert_y(),
            transition_wave: default_transition_wave(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Err(SwwwsError::Config(ConfigError::FileRead {
                path: config_path,
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            }));
        }
        
        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| SwwwsError::Config(ConfigError::FileRead {
                path: config_path.clone(),
                source: e,
            }))?;
        
        // Debug raw TOML content
        log::info!("Raw TOML content: {:?}", content);
        if let Some(line) = content.lines().find(|l| l.contains("monitor_behavior")) {
            log::info!("Monitor behavior line: {:?}", line);
        }
        
        let mut config: Config = toml::from_str(&content)
            .map_err(|e| SwwwsError::Config(ConfigError::TomlParse {
                message: e.to_string(),
            }))?;
        
        // Manual parsing fix for monitor_behavior
        if let Some(line) = content.lines().find(|l| l.contains("monitor_behavior")) {
            if line.contains("\"Synchronized\"") {
                log::info!("Manually setting monitor_behavior to Synchronized");
                config.monitor_behavior = MonitorBehavior::Synchronized;
            } else if line.contains("\"Independent\"") {
                log::info!("Manually setting monitor_behavior to Independent");
                config.monitor_behavior = MonitorBehavior::Independent;
            } else if line.contains("\"Grouped\"") {
                log::info!("Manually setting monitor_behavior to Grouped");
                // Use empty vec for now, groups will be handled by get_effective_monitor_behavior
                config.monitor_behavior = MonitorBehavior::Grouped(vec![]);
            }
        }
        
        // Debug log the parsed config
        log::info!("Parsed monitor_behavior from config: {:?}", config.monitor_behavior);
        log::info!("Effective monitor behavior: {:?}", config.get_effective_monitor_behavior());
        
        // Validate the configuration
        config.validate()?;
        
        Ok(config)
    }
    
    fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| SwwwsError::Config(ConfigError::NoConfigDir))?
            .join("swwws");
        
        Ok(config_dir.join("config.toml"))
    }
    
    pub fn get_output_config(&self, output_name: &str) -> OutputConfig {
        let mut config = self.any.clone();
        
        // Apply global defaults first
        config.merge_from_global(&self.global);
        
        // Then apply output-specific config if it exists
        if let Some(output_config) = self.outputs.get(output_name) {
            config.merge_from_output(output_config);
        }
        
        config
    }
    
    pub fn get_effective_monitor_behavior(&self) -> MonitorBehavior {
        match (&self.monitor_behavior, &self.monitor_groups) {
            (MonitorBehavior::Grouped(_), Some(groups)) => {
                MonitorBehavior::Grouped(groups.clone())
            }
            (MonitorBehavior::Grouped(_), None) => {
                log::warn!("Monitor behavior set to 'Grouped' but no monitor_groups defined, falling back to Independent");
                MonitorBehavior::Independent
            }
            (behavior, _) => behavior.clone()
        }
    }
    
    fn validate(&self) -> Result<()> {
        // Validate global configuration
        self.global.validate()?;
        
        // Validate any configuration
        self.any.validate()?;
        
        // Validate all output configurations
        for (output_name, output_config) in &self.outputs {
            output_config.validate()
                .map_err(|e| SwwwsError::Config(ConfigError::Validation {
                    message: format!("Output '{}': {}", output_name, e),
                }))?;
        }
        
        // Validate monitor behavior and groups
        self.validate_monitor_behavior()?;
        
        Ok(())
    }
    
    fn validate_monitor_behavior(&self) -> Result<()> {
        if let Some(groups) = &self.monitor_groups {
            // Check that groups is not empty
            if groups.is_empty() {
                return Err(SwwwsError::Config(ConfigError::Validation {
                    message: "monitor_groups cannot be empty".to_string(),
                }));
            }
            
            // Check that no group is empty
            for (i, group) in groups.iter().enumerate() {
                if group.is_empty() {
                    return Err(SwwwsError::Config(ConfigError::Validation {
                        message: format!("Group {} is empty", i),
                    }));
                }
            }
            
            // Check for duplicate outputs across groups
            let mut all_outputs = std::collections::HashSet::new();
            for (_i, group) in groups.iter().enumerate() {
                for output in group {
                    if !all_outputs.insert(output.clone()) {
                        return Err(SwwwsError::Config(ConfigError::Validation {
                            message: format!("Output '{}' appears in multiple groups", output),
                        }));
                    }
                }
            }
            
            // If monitor_behavior is not Grouped but groups are defined, warn
            if !matches!(self.monitor_behavior, MonitorBehavior::Grouped(_)) {
                log::warn!("monitor_groups defined but monitor_behavior is not 'Grouped'");
            }
        } else if matches!(self.monitor_behavior, MonitorBehavior::Grouped(_)) {
            // monitor_behavior is Grouped but no groups defined - this will fall back to Independent
            log::warn!("monitor_behavior is 'Grouped' but no monitor_groups defined");
        }
        
        Ok(())
    }
}

impl GlobalConfig {
    fn validate(&self) -> Result<()> {
        // Validate duration
        if self.duration < Duration::from_secs(1) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "duration".to_string(),
                value: format!("{:?}", self.duration),
            }));
        }
        
        // Validate queue size
        if self.queue_size == 0 {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "queue_size".to_string(),
                value: self.queue_size.to_string(),
            }));
        }
        
        // Validate transition step
        if self.transition_step == 0 {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_step".to_string(),
                value: self.transition_step.to_string(),
            }));
        }
        
        // Validate transition angle
        if !(0.0..=360.0).contains(&self.transition_angle) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_angle".to_string(),
                value: self.transition_angle.to_string(),
            }));
        }
        
        // Validate transition duration
        if self.transition_duration < Duration::from_millis(1) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_duration".to_string(),
                value: format!("{:?}", self.transition_duration),
            }));
        }
        
        Ok(())
    }
}

impl OutputConfig {
    pub fn merge(&mut self, other: &OutputConfig) {
        if self.path.is_none() {
            self.path = other.path.clone();
        }
        if self.duration == default_duration() {
            self.duration = other.duration;
        }
        if self.queue_size == default_queue_size() {
            self.queue_size = other.queue_size;
        }
        if self.sorting == default_sorting() {
            self.sorting = other.sorting.clone();
        }
        if self.transition_type == default_transition_type() {
            self.transition_type = other.transition_type.clone();
        }
        if self.transition_step == default_transition_step() {
            self.transition_step = other.transition_step;
        }
        if self.transition_angle == default_transition_angle() {
            self.transition_angle = other.transition_angle;
        }
        if self.transition_pos == default_transition_pos() {
            self.transition_pos = other.transition_pos.clone();
        }
        if self.transition_bezier == default_transition_bezier() {
            self.transition_bezier = other.transition_bezier.clone();
        }
        if self.transition_duration == default_transition_duration() {
            self.transition_duration = other.transition_duration;
        }
        if self.resize == default_resize() {
            self.resize = other.resize.clone();
        }
        if self.fill_color == default_fill_color() {
            self.fill_color = other.fill_color.clone();
        }
        if self.filter == default_filter() {
            self.filter = other.filter.clone();
        }
        if self.invert_y == default_invert_y() {
            self.invert_y = other.invert_y;
        }
        if self.transition_wave == default_transition_wave() {
            self.transition_wave = other.transition_wave.clone();
        }
    }
    
    pub fn merge_from_global(&mut self, global: &GlobalConfig) {
        if self.duration == default_duration() {
            self.duration = global.duration;
        }
        if self.queue_size == default_queue_size() {
            self.queue_size = global.queue_size;
        }
        if self.sorting == default_sorting() {
            self.sorting = global.sorting.clone();
        }
        if self.transition_type == default_transition_type() {
            self.transition_type = global.transition_type.clone();
        }
        if self.transition_step == default_transition_step() {
            self.transition_step = global.transition_step;
        }
        if self.transition_angle == default_transition_angle() {
            self.transition_angle = global.transition_angle;
        }
        if self.transition_pos == default_transition_pos() {
            self.transition_pos = global.transition_pos.clone();
        }
        if self.transition_bezier == default_transition_bezier() {
            self.transition_bezier = global.transition_bezier.clone();
        }
        if self.transition_duration == default_transition_duration() {
            self.transition_duration = global.transition_duration;
        }
        if self.resize == default_resize() {
            self.resize = global.resize.clone();
        }
        if self.fill_color == default_fill_color() {
            self.fill_color = global.fill_color.clone();
        }
        if self.filter == default_filter() {
            self.filter = global.filter.clone();
        }
        if self.invert_y == default_invert_y() {
            self.invert_y = global.invert_y;
        }
        if self.transition_wave == default_transition_wave() {
            self.transition_wave = global.transition_wave.clone();
        }
    }
    
    pub fn merge_from_output(&mut self, other: &OutputConfig) {
        // Always override with output-specific values
        self.duration = other.duration;
        self.queue_size = other.queue_size;
        self.sorting = other.sorting.clone();
        self.transition_type = other.transition_type.clone();
        self.transition_step = other.transition_step;
        self.transition_angle = other.transition_angle;
        self.transition_pos = other.transition_pos.clone();
        self.transition_bezier = other.transition_bezier.clone();
        self.transition_duration = other.transition_duration;
        self.resize = other.resize.clone();
        self.fill_color = other.fill_color.clone();
        self.filter = other.filter.clone();
        self.invert_y = other.invert_y;
        self.transition_wave = other.transition_wave.clone();
        // Always override path if it's set
        if other.path.is_some() {
            self.path = other.path.clone();
        }
    }
    
    fn validate(&self) -> Result<()> {
        // Validate duration
        if self.duration < Duration::from_secs(1) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "duration".to_string(),
                value: format!("{:?}", self.duration),
            }));
        }
        
        // Validate queue size
        if self.queue_size == 0 {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "queue_size".to_string(),
                value: self.queue_size.to_string(),
            }));
        }
        
        // Validate transition step
        if self.transition_step == 0 {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_step".to_string(),
                value: self.transition_step.to_string(),
            }));
        }
        
        // Validate transition angle
        if !(0.0..=360.0).contains(&self.transition_angle) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_angle".to_string(),
                value: self.transition_angle.to_string(),
            }));
        }
        
        // Validate transition duration
        if self.transition_duration < Duration::from_millis(1) {
            return Err(SwwwsError::Config(ConfigError::InvalidValue {
                field: "transition_duration".to_string(),
                value: format!("{:?}", self.transition_duration),
            }));
        }
        
        Ok(())
    }
}

impl Clone for OutputConfig {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            duration: self.duration,
            queue_size: self.queue_size,
            sorting: self.sorting.clone(),
            transition_type: self.transition_type.clone(),
            transition_step: self.transition_step,
            transition_angle: self.transition_angle,
            transition_pos: self.transition_pos.clone(),
            transition_bezier: self.transition_bezier.clone(),
            transition_duration: self.transition_duration,
            resize: self.resize.clone(),
            fill_color: self.fill_color.clone(),
            filter: self.filter.clone(),
            invert_y: self.invert_y,
            transition_wave: self.transition_wave.clone(),
        }
    }
}

impl Clone for GlobalConfig {
    fn clone(&self) -> Self {
        Self {
            duration: self.duration,
            queue_size: self.queue_size,
            sorting: self.sorting.clone(),
            transition_type: self.transition_type.clone(),
            transition_step: self.transition_step,
            transition_angle: self.transition_angle,
            transition_pos: self.transition_pos.clone(),
            transition_bezier: self.transition_bezier.clone(),
            transition_duration: self.transition_duration,
            resize: self.resize.clone(),
            fill_color: self.fill_color.clone(),
            filter: self.filter.clone(),
            invert_y: self.invert_y,
            transition_wave: self.transition_wave.clone(),
        }
    }
}

mod monitor_behavior_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    #[test]
    fn test_duration_deserialization() {
        let toml_str = r#"
            [global]
            duration = "3m"
            transition_duration = "500ms"
        "#;
        
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.global.duration, Duration::from_secs(180));
        assert_eq!(config.global.transition_duration, Duration::from_millis(500));
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Test valid configuration
        assert!(config.validate().is_ok());
        
        // Test invalid duration
        config.global.duration = Duration::from_secs(0);
        assert!(config.validate().is_err());
        
        // Reset and test invalid queue size
        config.global.duration = Duration::from_secs(300);
        config.global.queue_size = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_output_config_merge() {
        let global = GlobalConfig {
            duration: Duration::from_secs(300),
            queue_size: 10,
            sorting: Sorting::Random,
            transition_type: "wipe".to_string(),
            transition_step: 90,
            transition_angle: 90.0,
            transition_pos: "center".to_string(),
            transition_bezier: "0.25,0.1,0.25,1".to_string(),
            transition_duration: Duration::from_millis(500),
        };
        
        let mut output = OutputConfig {
            path: Some("/test/path".to_string()),
            duration: Duration::from_secs(600), // Override global
            queue_size: 5, // Override global
            sorting: Sorting::Ascending, // Override global
            transition_type: "fade".to_string(), // Override global
            transition_step: 45, // Override global
            transition_angle: 45.0, // Override global
            transition_pos: "top-left".to_string(), // Override global
            transition_bezier: "0.5,0.5,0.5,0.5".to_string(), // Override global
            transition_duration: Duration::from_millis(1000), // Override global
        };
        
        output.merge_from_global(&global);
        
        // Should keep output-specific values
        assert_eq!(output.duration, Duration::from_secs(600));
        assert_eq!(output.queue_size, 5);
        assert_eq!(output.sorting, Sorting::Ascending);
        assert_eq!(output.transition_type, "fade");
        assert_eq!(output.transition_step, 45);
        assert_eq!(output.transition_angle, 45.0);
        assert_eq!(output.transition_pos, "top-left");
        assert_eq!(output.transition_bezier, "0.5,0.5,0.5,0.5");
        assert_eq!(output.transition_duration, Duration::from_millis(1000));
    }

    #[test]
    fn test_output_config_merge_defaults() {
        let global = GlobalConfig {
            duration: Duration::from_secs(300),
            queue_size: 10,
            sorting: Sorting::Random,
            transition_type: "wipe".to_string(),
            transition_step: 90,
            transition_angle: 90.0,
            transition_pos: "center".to_string(),
            transition_bezier: "0.25,0.1,0.25,1".to_string(),
            transition_duration: Duration::from_millis(500),
        };
        
        let mut output = OutputConfig::default();
        output.merge_from_global(&global);
        
        // Should inherit global values
        assert_eq!(output.duration, Duration::from_secs(300));
        assert_eq!(output.queue_size, 10);
        assert_eq!(output.sorting, Sorting::Random);
        assert_eq!(output.transition_type, "wipe");
        assert_eq!(output.transition_step, 90);
        assert_eq!(output.transition_angle, 90.0);
        assert_eq!(output.transition_pos, "center");
        assert_eq!(output.transition_bezier, "0.25,0.1,0.25,1");
        assert_eq!(output.transition_duration, Duration::from_millis(500));
    }

    #[test]
    fn test_config_load_from_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
            [global]
            duration = "3m"
            queue_size = 5
            sorting = "Random"
            transition_type = "wipe"
            transition_step = 90
            transition_angle = 90.0
            transition_pos = "center"
            transition_bezier = "0.25,0.1,0.25,1"
            transition_duration = "500ms"

            ["HDMI-A-1"]
            path = "/test/path"
            duration = "5m"
            transition_type = "fade"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        // Mock the config_path function to return our test file
        let config = Config::load_from_path(&config_path).unwrap();
        
        assert_eq!(config.global.duration, Duration::from_secs(180));
        assert_eq!(config.global.queue_size, 5);
        assert_eq!(config.global.sorting, Sorting::Random);
        
        let output_config = config.get_output_config("HDMI-A-1");
        assert_eq!(output_config.path, Some("/test/path".to_string()));
        assert_eq!(output_config.duration, Duration::from_secs(300));
        assert_eq!(output_config.transition_type, "fade");
    }

    #[test]
    fn test_config_load_nonexistent_file() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.toml");
        
        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SwwwsError::Config(ConfigError::FileRead { .. }) => {},
            _ => panic!("Expected ConfigError::FileRead"),
        }
    }

    #[test]
    fn test_config_load_invalid_toml() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("invalid.toml");
        
        let invalid_content = r#"
            [global]
            duration = "invalid"
        "#;
        
        fs::write(&config_path, invalid_content).unwrap();
        
        let result = Config::load_from_path(&config_path);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            SwwwsError::Config(ConfigError::TomlParse { .. }) => {},
            _ => panic!("Expected ConfigError::TomlParse"),
        }
    }
}

impl Config {
    // Helper function for testing
    #[cfg(test)]
    fn load_from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(SwwwsError::Config(ConfigError::FileRead {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "File not found"),
            }));
        }
        
        let content = std::fs::read_to_string(path)
            .map_err(|e| SwwwsError::Config(ConfigError::FileRead {
                path: path.to_path_buf(),
                source: e,
            }))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| SwwwsError::Config(ConfigError::TomlParse {
                message: e.to_string(),
            }))?;
        
        config.validate()?;
        Ok(config)
    }
}
