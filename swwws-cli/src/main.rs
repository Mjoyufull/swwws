use clap::{Parser, Subcommand};
use swwws_common::{IpcClient, IpcCommand, IpcResponse};

#[derive(Parser)]
#[command(name = "swwws-cli")]
#[command(about = "swwws-cli (swww slideshow daemon control)")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the swwws daemon
    Daemon,
    
    /// Advance to next wallpaper
    Next {
        /// Specific output to advance
        #[arg(long)]
        output: Option<String>,
    },
    
    /// Go to previous wallpaper
    Previous {
        /// Specific output to go back
        #[arg(long)]
        output: Option<String>,
    },
    
    /// Pause the slideshow
    Pause,
    
    /// Resume the slideshow
    Resume,
    
    /// Toggle pause state
    TogglePause,
    
    /// Reload configuration
    Reload,
    
    /// Show current status
    Status,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Daemon => {
            println!("To start the daemon, run: swwws-daemon");
            println!("Or use systemctl --user start swwws if installed via install.sh");
        }
        
        Commands::Next { output } => {
            let client = IpcClient::new();
            let command = IpcCommand::Next { output };
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Previous { output } => {
            let client = IpcClient::new();
            let command = IpcCommand::Previous { output };
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Pause => {
            let client = IpcClient::new();
            let command = IpcCommand::Pause;
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Resume => {
            let client = IpcClient::new();
            let command = IpcCommand::Resume;
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::TogglePause => {
            let client = IpcClient::new();
            let command = IpcCommand::TogglePause;
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Reload => {
            let client = IpcClient::new();
            let command = IpcCommand::Reload;
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Commands::Status => {
            let client = IpcClient::new();
            let command = IpcCommand::Status;
            
            match client.send_command(command) {
                Ok(response) => print_response(response),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }

    Ok(())
}

fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        if secs == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m{}s", minutes, secs)
        }
    } else {
        let hours = seconds / 3600;
        let remaining = seconds % 3600;
        let minutes = remaining / 60;
        if minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h{}m", hours, minutes)
        }
    }
}

fn print_response(response: IpcResponse) {
    match response {
        IpcResponse::Success { message } => {
            println!("✓ {}", message);
        }
        
        IpcResponse::Error { message } => {
            eprintln!("✗ Error: {}", message);
            std::process::exit(1);
        }
        
        IpcResponse::Status { outputs, paused } => {
            if outputs.is_empty() {
                println!("No outputs found");
                return;
            }
            
            println!("swwws Status:");
            println!("=============");
            println!("Global State: {}", if paused { "PAUSED" } else { "RUNNING" });
            println!();
            
            for output in outputs {
                let status = if output.paused { "PAUSED" } else { "RUNNING" };
                let timer_str = if let Some(remaining) = output.timer_remaining {
                    if remaining > 0 {
                        format_duration(remaining)
                    } else {
                        "ready".to_string()
                    }
                } else {
                    "no timer".to_string()
                };
                
                let current_image = output.current_image.as_deref()
                    .map(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                    })
                    .unwrap_or("None");
                
                println!("{}: {} | {} | {}/{} | {}", 
                    output.name,
                    status,
                    current_image,
                    output.queue_position + 1,
                    output.queue_size,
                    timer_str
                );
            }
        }
    }
}
