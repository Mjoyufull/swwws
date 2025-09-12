use anyhow::{anyhow, Result};
use rustix::net::{self, RecvFlags, SendFlags, SocketAddrUnix, SocketType, AddressFamily};
use rustix::fd::OwnedFd;
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use std::thread;
use rustix::io::{IoSlice, IoSliceMut};
use std::os::fd::AsRawFd;

/// Represents a connection to the swww daemon
pub struct SwwwClient {
    socket: OwnedFd,
}

impl SwwwClient {
    /// Connect to the swww daemon socket
    pub fn connect() -> Result<Self> {
        let socket_path = Self::get_socket_path();
        
        let socket = net::socket_with(
            AddressFamily::UNIX,
            SocketType::STREAM,
            net::SocketFlags::CLOEXEC,
            None,
        )?;
        
        let addr = SocketAddrUnix::new(&socket_path)
            .map_err(|_| anyhow!("Failed to create socket address for path: {:?}", socket_path))?;
        
        // Try connecting with retries like swww does
        let mut last_error = None;
        for attempt in 1..=5 {
            match net::connect_unix(&socket, &addr) {
                Ok(()) => {
                    // Set socket timeout
                    let timeout = Duration::from_secs(5);
                    net::sockopt::set_socket_timeout(
                        &socket,
                        net::sockopt::Timeout::Recv,
                        Some(timeout),
                    )?;
                    return Ok(Self { socket });
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < 5 {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        
        Err(anyhow!(
            "Failed to connect to swww daemon at {:?}: {:?}",
            socket_path,
            last_error.unwrap()
        ))
    }
    
    /// Get the socket path for swww daemon
    fn get_socket_path() -> PathBuf {
        let mut runtime = env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let mut p = PathBuf::from("/run/user");
                let uid = rustix::process::getuid();
                p.push(format!("{}", uid.as_raw()));
                p
            });

        let display = if let Ok(wayland_socket) = env::var("WAYLAND_DISPLAY") {
            let mut i = 0;
            // If WAYLAND_DISPLAY is a full path, use only its final component
            for (j, ch) in wayland_socket.bytes().enumerate().rev() {
                if ch == b'/' {
                    i = j + 1;
                    break;
                }
            }
            format!("{}-swww-daemon", &wayland_socket[i..])
        } else {
            "wayland-0-swww-daemon".to_string()
        };

        runtime.push(display);
        runtime.set_extension(".sock");
        runtime
    }
    
    /// Query the swww daemon for output information
    pub fn query(&self) -> Result<Vec<SwwwOutput>> {
        // Send query request (code 1)
        self.send_request(1, None)?;
        
        // Receive response
        let response = self.receive_response()?;
        
        // Parse response based on swww protocol
        match response.code {
            8 => {
                // ResInfo - parse the background info
                if let Some(data) = response.data {
                    self.parse_bg_info(&data)
                } else {
                    Err(anyhow!("Expected data with ResInfo response"))
                }
            }
            _ => Err(anyhow!("Unexpected response code: {}", response.code)),
        }
    }
    
    /// Set wallpaper on specified outputs  
    pub fn set_wallpaper(&self, image_path: &str, outputs: &[String], _transition: SwwwTransition) -> Result<()> {
        // For now, use the subprocess approach to avoid crashing swww-daemon
        // The socket protocol is complex and our implementation was causing crashes
        self.set_wallpaper_subprocess(image_path, outputs)
    }
    
    fn set_wallpaper_subprocess(&self, image_path: &str, outputs: &[String]) -> Result<()> {
        use std::process::Command;
        
        // Ensure swww binary exists
        let swww_path = which::which("swww")
            .map_err(|_| anyhow!("swww binary not found in PATH"))?;
            
        for output_name in outputs {
            let mut cmd = Command::new(&swww_path);
            cmd.arg("img")
                .arg("-o")
                .arg(output_name)
                .arg(image_path);
                
            // Set environment variables to match current session
            if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
                cmd.env("WAYLAND_DISPLAY", display);
            }
            if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
                cmd.env("XDG_RUNTIME_DIR", runtime_dir);
            }
            
            log::debug!("Executing swww command: {:?}", cmd);
            
            // Just run it and wait for completion - no timeout
            let output = cmd.output()
                .map_err(|e| anyhow!("Failed to execute swww command: {}", e))?;
                
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("swww command failed for output {}: {}", output_name, stderr);
            } else {
                log::info!("Successfully set wallpaper for output: {}", output_name);
            }
        }
        
        Ok(())
    }
    
    fn send_request(&self, code: u64, _data: Option<Vec<u8>>) -> Result<()> {
        let mut payload = [0u8; 16];
        payload[0..8].copy_from_slice(&code.to_ne_bytes());
        // For now, send 0 length (no shared memory)
        payload[8..16].copy_from_slice(&0u64.to_ne_bytes());
        
        let iov = IoSlice::new(&payload);
        let written = net::sendmsg(&self.socket, &[iov], &mut net::SendAncillaryBuffer::new(&mut []), SendFlags::empty())?;
        
        if written != payload.len() {
            return Err(anyhow!("Failed to send complete message"));
        }
        
        Ok(())
    }
    
    fn receive_response(&self) -> Result<SwwwResponse> {
        let mut buf = [0u8; 16];
        let mut ancillary_buf = [0u8; rustix::cmsg_space!(ScmRights(1))];
        let mut control = net::RecvAncillaryBuffer::new(&mut ancillary_buf);
        
        // Try receiving with retries like swww does
        for _ in 0..5 {
            let iov = IoSliceMut::new(&mut buf);
            match net::recvmsg(&self.socket, &mut [iov], &mut control, RecvFlags::WAITALL) {
                Ok(_) => break,
                Err(e) if matches!(e, rustix::io::Errno::WOULDBLOCK | rustix::io::Errno::INTR) => {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(e) => return Err(anyhow!("Failed to receive response: {}", e)),
            }
        }
        
        let code = u64::from_ne_bytes(buf[0..8].try_into().unwrap());
        let len = u64::from_ne_bytes(buf[8..16].try_into().unwrap()) as usize;
        
        let data = if len > 0 {
            // Get the file descriptor from ancillary data
            let fd = control
                .drain()
                .next()
                .and_then(|msg| match msg {
                    net::RecvAncillaryMessage::ScmRights(mut iter) => iter.next(),
                    _ => None,
                })
                .ok_or_else(|| anyhow!("Expected file descriptor but didn't receive one"))?;
                
            // Read data from the memory mapped file
            // This is simplified - a proper implementation would use mmap
            Some(self.read_fd_data(fd, len)?)
        } else {
            None
        };
        
        Ok(SwwwResponse { code, data })
    }
    
    fn read_fd_data(&self, fd: OwnedFd, len: usize) -> Result<Vec<u8>> {
        unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            );
            
            if ptr == libc::MAP_FAILED {
                return Err(anyhow!("Failed to mmap file descriptor"));
            }
            
            let slice = std::slice::from_raw_parts(ptr as *const u8, len);
            let data = slice.to_vec();
            
            libc::munmap(ptr, len);
            
            Ok(data)
        }
    }
    
    fn parse_bg_info(&self, data: &[u8]) -> Result<Vec<SwwwOutput>> {
        if data.is_empty() {
            return Err(anyhow!("No data received from swww daemon"));
        }
        
        let mut outputs = Vec::new();
        let count = data[0] as usize;
        let mut offset = 1;
        
        for _ in 0..count {
            if offset >= data.len() {
                break;
            }
            
            // Parse name length and name
            if offset + 4 > data.len() {
                break;
            }
            let name_len = u32::from_ne_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]) as usize;
            offset += 4;
            
            if offset + name_len > data.len() {
                break;
            }
            let name = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
            offset += name_len;
            
            // Parse dimensions
            if offset + 8 > data.len() {
                break;
            }
            let width = u32::from_ne_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]);
            offset += 4;
            let height = u32::from_ne_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]);
            offset += 4;
            
            // Parse scale factor (skip the discriminant byte and get the value)
            if offset + 5 > data.len() {
                break;
            }
            offset += 1; // Skip discriminant
            let scale_raw = i32::from_ne_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
            ]);
            offset += 4;
            let scale = scale_raw as f32;
            
            // Skip the rest of the BgInfo structure (image info and pixel format)
            // This is a simplified parser - we just need the basic output info
            if offset < data.len() {
                // Skip image info
                if data[offset] == 0 {
                    // Color - skip 4 bytes
                    offset += 5;
                } else {
                    // Image path - skip length + path
                    offset += 1;
                    if offset + 4 <= data.len() {
                        let path_len = u32::from_ne_bytes([
                            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
                        ]) as usize;
                        offset += 4 + path_len;
                    }
                }
                // Skip pixel format byte
                if offset < data.len() {
                    offset += 1;
                }
            }
            
            outputs.push(SwwwOutput {
                name,
                width,
                height,
                scale,
            });
        }
        
        if outputs.is_empty() {
            Err(anyhow!("No valid outputs parsed from swww daemon response"))
        } else {
            Ok(outputs)
        }
    }
}

#[derive(Debug)]
pub struct SwwwOutput {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

#[derive(Debug)]
pub struct SwwwTransition {
    pub transition_type: String,
    pub duration: f32,
    pub step: u8,
    pub fps: u16,
    pub angle: f64,
    pub pos_x: f32,
    pub pos_y: f32,
    pub bezier: (f32, f32, f32, f32),
    pub wave: (f32, f32),
    pub invert_y: bool,
}

impl Default for SwwwTransition {
    fn default() -> Self {
        Self {
            transition_type: "outer".to_string(),
            duration: 0.5,
            step: 90,
            fps: 30,
            angle: 0.0,
            pos_x: 0.5,
            pos_y: 0.5,
            bezier: (0.0, 0.0, 1.0, 1.0),
            wave: (20.0, 20.0),
            invert_y: false,
        }
    }
}

struct SwwwResponse {
    code: u64,
    data: Option<Vec<u8>>,
}
