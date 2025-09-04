pub mod queue;
pub mod image_discovery;
pub mod swww;
pub mod command_builder;
pub mod executor;
pub mod duration;
pub mod ipc;
pub mod state;
pub mod error;

pub use queue::{Queue, Sorting};
pub use image_discovery::ImageDiscovery;
pub use swww::SwwwIntegration;
pub use command_builder::CommandBuilder;
pub use executor::ProcessExecutor;
pub use duration::parse_duration;
pub use ipc::{IpcClient, IpcServer, IpcCommand, IpcResponse, OutputStatus};
pub use state::{DaemonState, OutputState};
pub use error::{SwwwsError, Result, ErrorReporting};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum MonitorBehavior {
    Independent,   // Each monitor has its own queue and timing
    Synchronized,  // All monitors show same image at same time
    Grouped(Vec<Vec<String>>), // Custom groups of monitors
}

impl Default for MonitorBehavior {
    fn default() -> Self {
        MonitorBehavior::Independent
    }
}
