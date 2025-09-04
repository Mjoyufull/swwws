use std::path::PathBuf;
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcCommand {
    Next { output: Option<String> },
    Previous { output: Option<String> },
    Pause,
    Resume,
    TogglePause,
    Reload,
    Status,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Success { message: String },
    Error { message: String },
    Status {
        outputs: Vec<OutputStatus>,
        paused: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputStatus {
    pub name: String,
    pub current_image: Option<String>,
    pub queue_position: usize,
    pub queue_size: usize,
    pub timer_remaining: Option<u64>, // seconds
    pub paused: bool,
}

pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    pub fn new() -> Self {
        let socket_path = dirs::runtime_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("swwws.sock");
        Self { socket_path }
    }

    pub fn send_command(&self, command: IpcCommand) -> Result<IpcResponse> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("Failed to connect to swwws daemon at {:?}", self.socket_path))?;

        let command_json = serde_json::to_string(&command)
            .with_context(|| "Failed to serialize command")?;
        
        stream.write_all(command_json.as_bytes())
            .with_context(|| "Failed to send command to daemon")?;
        stream.shutdown(std::net::Shutdown::Write)
            .with_context(|| "Failed to shutdown write stream")?;

        let mut response = String::new();
        stream.read_to_string(&mut response)
            .with_context(|| "Failed to read response from daemon")?;

        let ipc_response: IpcResponse = serde_json::from_str(&response)
            .with_context(|| "Failed to deserialize response")?;

        Ok(ipc_response)
    }
}

pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn new() -> Self {
        let socket_path = dirs::runtime_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("swwws.sock");
        Self { socket_path }
    }

    pub fn start<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(IpcCommand) -> Result<IpcResponse> + Send + Clone + 'static,
    {
        // Remove existing socket if it exists
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)
                .with_context(|| "Failed to remove existing socket")?;
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| "Failed to create socket directory")?;
        }

        let listener = std::os::unix::net::UnixListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind to socket {:?}", self.socket_path))?;

        log::info!("IPC server listening on {:?}", self.socket_path);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = handler.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = Self::handle_connection(stream, &handler) {
                            log::error!("Error handling IPC connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("Error accepting IPC connection: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection<F>(
        mut stream: std::os::unix::net::UnixStream,
        handler: &F,
    ) -> Result<()>
    where
        F: Fn(IpcCommand) -> Result<IpcResponse>,
    {
        let mut command_json = String::new();
        stream.read_to_string(&mut command_json)
            .with_context(|| "Failed to read command from client")?;

        let command: IpcCommand = serde_json::from_str(&command_json)
            .with_context(|| "Failed to deserialize command")?;

        let response = handler(command)
            .unwrap_or_else(|e| IpcResponse::Error { message: e.to_string() });

        let response_json = serde_json::to_string(&response)
            .with_context(|| "Failed to serialize response")?;

        stream.write_all(response_json.as_bytes())
            .with_context(|| "Failed to send response to client")?;

        Ok(())
    }
}
