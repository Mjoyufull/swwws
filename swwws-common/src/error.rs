use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for swwws operations
#[derive(Error, Debug)]
pub enum SwwwsError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Image discovery error: {0}")]
    ImageDiscovery(#[from] ImageDiscoveryError),

    #[error("Queue error: {0}")]
    Queue(#[from] QueueError),

    #[error("swww integration error: {0}")]
    Swww(#[from] SwwwError),

    #[error("IPC error: {0}")]
    Ipc(#[from] IpcError),

    #[error("State persistence error: {0}")]
    State(#[from] StateError),

    #[error("Process execution error: {0}")]
    Process(#[from] ProcessError),

    #[error("System error: {0}")]
    System(#[from] SystemError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}

/// Configuration-related errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {path:?}")]
    FileRead { path: PathBuf, source: std::io::Error },

    #[error("Failed to parse TOML configuration: {message}")]
    TomlParse { message: String },

    #[error("Configuration validation failed: {message}")]
    Validation { message: String },

    #[error("Missing required configuration: {field}")]
    MissingField { field: String },

    #[error("Invalid configuration value for {field}: {value}")]
    InvalidValue { field: String, value: String },

    #[error("Could not determine config directory")]
    NoConfigDir,
}

/// Image discovery errors
#[derive(Error, Debug)]
pub enum ImageDiscoveryError {
    #[error("Failed to read directory: {path:?}")]
    DirectoryRead { path: PathBuf, source: std::io::Error },

    #[error("No images found in directory: {path:?}")]
    NoImagesFound { path: PathBuf },

    #[error("Failed to access image file: {path:?}")]
    FileAccess { path: PathBuf, source: std::io::Error },

    #[error("Unsupported image format: {path:?}")]
    UnsupportedFormat { path: PathBuf },

    #[error("Image file is corrupted or invalid: {path:?}")]
    CorruptedImage { path: PathBuf },
}

/// Queue management errors
#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Queue is empty")]
    EmptyQueue,

    #[error("Invalid queue position: {position} (max: {max})")]
    InvalidPosition { position: usize, max: usize },

    #[error("Failed to restore queue from state: {message}")]
    StateRestore { message: String },

    #[error("Queue sorting error: {message}")]
    Sorting { message: String },
}

/// swww integration errors
#[derive(Error, Debug)]
pub enum SwwwError {
    #[error("swww daemon not found or not running")]
    DaemonNotFound,

    #[error("Failed to discover swww outputs")]
    OutputDiscovery,

    #[error("swww command execution failed: {command:?}")]
    CommandExecution { command: String, stderr: String },

    #[error("Invalid swww output: {output}")]
    InvalidOutput { output: String },

    #[error("swww process error: {message}")]
    Process { message: String },
}

/// IPC communication errors
#[derive(Error, Debug)]
pub enum IpcError {
    #[error("Failed to create IPC socket")]
    SocketCreation,

    #[error("Failed to connect to daemon")]
    Connection,

    #[error("Failed to send IPC message")]
    Send,

    #[error("Failed to receive IPC response")]
    Receive,

    #[error("IPC timeout")]
    Timeout,

    #[error("Invalid IPC message format")]
    InvalidMessage,

    #[error("Daemon not responding")]
    DaemonUnresponsive,
}

/// State persistence errors
#[derive(Error, Debug)]
pub enum StateError {
    #[error("Failed to read state file: {path:?}")]
    FileRead { path: PathBuf, source: std::io::Error },

    #[error("Failed to write state file: {path:?}")]
    FileWrite { path: PathBuf, source: std::io::Error },

    #[error("Failed to serialize state")]
    Serialization,

    #[error("Failed to deserialize state")]
    Deserialization,

    #[error("State file is corrupted: {message}")]
    Corrupted { message: String },

    #[error("Failed to create state directory")]
    DirectoryCreation,
}

/// Process execution errors
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Command execution failed: {command:?}")]
    Execution { command: String, source: std::io::Error },

    #[error("Command returned non-zero exit code: {code}")]
    NonZeroExit { code: i32, stderr: String },

    #[error("Command timed out")]
    Timeout,

    #[error("Command was killed")]
    Killed,
}

/// System-level errors
#[derive(Error, Debug)]
pub enum SystemError {
    #[error("Failed to get current time")]
    Time,

    #[error("Failed to create temporary file")]
    TempFile,

    #[error("Failed to access file system")]
    FileSystem,

    #[error("Insufficient permissions")]
    Permissions,

    #[error("System resource limit exceeded")]
    ResourceLimit,
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid path: {path:?}")]
    InvalidPath { path: PathBuf },

    #[error("Invalid duration: {duration}")]
    InvalidDuration { duration: String },

    #[error("Invalid output name: {output}")]
    InvalidOutput { output: String },

    #[error("Invalid image format: {format}")]
    InvalidImageFormat { format: String },

    #[error("Validation failed: {message}")]
    General { message: String },
}

// Helper traits for error conversion
pub trait ErrorContext<T> {
    fn with_context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<SwwwsError>,
{
    fn with_context<C>(self, _context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.into())
    }
}

// Convenience type alias
pub type Result<T> = std::result::Result<T, SwwwsError>;

// Error conversion implementations
impl From<std::io::Error> for SwwwsError {
    fn from(_err: std::io::Error) -> Self {
        SwwwsError::System(SystemError::FileSystem)
    }
}

impl From<serde_json::Error> for SwwwsError {
    fn from(_err: serde_json::Error) -> Self {
        SwwwsError::State(StateError::Serialization)
    }
}

impl From<toml::de::Error> for SwwwsError {
    fn from(err: toml::de::Error) -> Self {
        SwwwsError::Config(ConfigError::TomlParse {
            message: err.to_string(),
        })
    }
}

impl From<toml::ser::Error> for SwwwsError {
    fn from(err: toml::ser::Error) -> Self {
        SwwwsError::Config(ConfigError::TomlParse {
            message: err.to_string(),
        })
    }
}

// Error reporting utilities
pub trait ErrorReporting {
    fn log_error(&self, context: &str);
    fn user_friendly_message(&self) -> String;
}

impl ErrorReporting for SwwwsError {
    fn log_error(&self, context: &str) {
        log::error!("{}: {:?}", context, self);
    }

    fn user_friendly_message(&self) -> String {
        match self {
            SwwwsError::Config(ConfigError::FileRead { path, .. }) => {
                format!("Configuration file not found: {:?}", path)
            }
            SwwwsError::Config(ConfigError::TomlParse { message }) => {
                format!("Invalid configuration format: {}", message)
            }
            SwwwsError::ImageDiscovery(ImageDiscoveryError::NoImagesFound { path }) => {
                format!("No images found in directory: {:?}", path)
            }
            SwwwsError::Swww(SwwwError::DaemonNotFound) => {
                "swww daemon is not running. Please start swww-daemon first.".to_string()
            }
            SwwwsError::Ipc(IpcError::DaemonUnresponsive) => {
                "swwws daemon is not responding. Please restart the daemon.".to_string()
            }
            SwwwsError::State(StateError::FileRead { path, .. }) => {
                format!("Failed to read state file: {:?}", path)
            }
            _ => self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_config_error_user_friendly_message() {
        let error = ConfigError::FileRead {
            path: PathBuf::from("/nonexistent/config.toml"),
            source: io::Error::new(io::ErrorKind::NotFound, "File not found"),
        };
        let swwws_error = SwwwsError::Config(error);
        
        let message = swwws_error.user_friendly_message();
        assert!(message.contains("Configuration file not found"));
        assert!(message.contains("/nonexistent/config.toml"));
    }

    #[test]
    fn test_image_discovery_error_user_friendly_message() {
        let error = ImageDiscoveryError::NoImagesFound {
            path: PathBuf::from("/empty/directory"),
        };
        let swwws_error = SwwwsError::ImageDiscovery(error);
        
        let message = swwws_error.user_friendly_message();
        assert!(message.contains("No images found"));
        assert!(message.contains("/empty/directory"));
    }

    #[test]
    fn test_process_error_user_friendly_message() {
        let error = ProcessError::NonZeroExit {
            code: 1,
            stderr: "Socket file not found".to_string(),
        };
        let swwws_error = SwwwsError::Process(error);
        
        let message = swwws_error.user_friendly_message();
        assert!(message.contains("Command returned non-zero exit code"));
        assert!(message.contains("1"));
    }

    #[test]
    fn test_error_conversion() {
        // Test io::Error conversion
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let swwws_error: SwwwsError = io_error.into();
        
        match swwws_error {
            SwwwsError::System(SystemError::FileSystem) => {},
            _ => panic!("Expected SystemError::FileSystem"),
        }
    }

    #[test]
    fn test_validation_error() {
        let error = ValidationError::General {
            message: "Duration must be positive".to_string(),
        };
        let swwws_error = SwwwsError::Validation(error);
        
        let message = swwws_error.user_friendly_message();
        assert!(message.contains("Validation failed"));
        assert!(message.contains("Duration must be positive"));
    }

    #[test]
    fn test_error_context() {
        let error = ConfigError::FileRead {
            path: PathBuf::from("/test/config.toml"),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"),
        };
        
        let swwws_error = SwwwsError::Config(error);
        
        let message = swwws_error.user_friendly_message();
        assert!(message.contains("Configuration file not found"));
        assert!(message.contains("/test/config.toml"));
    }
}
