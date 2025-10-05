use anyhow::Result;
use swwws_config::Config;
use swwws_common::{
    ImageDiscovery, Queue, CommandBuilder, ProcessExecutor, IpcServer, IpcCommand, IpcResponse, OutputStatus, 
    DaemonState as PersistentState, ErrorReporting, MonitorBehavior,
};
use swwws_common::queue::Sorting;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use tokio::time::interval;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct MonitorGroup {
    name: String,
    outputs: Vec<String>,
    queue: Queue,
    timer: Instant,
}

#[derive(Debug)]
struct DaemonState {
    queues: HashMap<String, Queue>,
    timers: HashMap<String, Instant>,
    groups: Vec<MonitorGroup>,  // For grouped behavior
    shared_queue: Option<Queue>, // For synchronized behavior
    shared_timer: Option<Instant>, // For synchronized behavior
    paused: bool,
    persistent_state: PersistentState,
}

impl DaemonState {
    fn new() -> Result<Self> {
        let persistent_state = PersistentState::load(&PersistentState::get_state_file())
            .unwrap_or_else(|e| {
                log::warn!("Failed to load state, starting fresh: {}", e);
                PersistentState::new()
            });

        Ok(Self {
            queues: HashMap::new(),
            timers: HashMap::new(),
            groups: Vec::new(),
            shared_queue: None,
            shared_timer: None,
            paused: persistent_state.is_paused(),
            persistent_state,
        })
    }

    fn save_state(&mut self) -> Result<()> {
        // Sync queue state to persistent storage
        for (output_name, queue) in &self.queues {
            if let Some(current_image) = queue.current_image() {
                self.persistent_state.update_output_state(
                    output_name,
                    Some(current_image),
                    queue.current_position(),
                    queue.size(),
                    queue.get_sorting(),
                    &queue.get_all_images(),
                );
            }
        }

        self.persistent_state.set_paused(self.paused);

        // Save to file
        let state_file = PersistentState::get_state_file();
        self.persistent_state.save(&state_file)
            .map_err(|e| {
                log::error!("Failed to save state: {}", e);
                e
            })?;

        log::debug!("State saved successfully");
        Ok(())
    }

    fn restore_queue_from_state(&mut self, output_name: &str, discovered_images: Vec<PathBuf>) -> bool {
        // Don't restore individual queues if we're in synchronized mode
        if self.shared_queue.is_some() {
            log::info!("Skipping queue restoration for {} - synchronized mode active", output_name);
            return false;
        }
        
        if let Some(saved_state) = self.persistent_state.get_output_state(output_name) {
            log::info!("Attempting to restore queue for {} from saved state", output_name);

            // Compare against saved image list
            let discovered_strings: Vec<String> = discovered_images
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();

            match saved_state.sorting {
                Sorting::Random => {
                    // Random mode: restore current position if image still exists
                    if let Some(current_image) = &saved_state.current_image {
                        if discovered_strings.contains(current_image) {
                            if let Some(mut queue) = Queue::new(
                                saved_state.queue_size,
                                saved_state.sorting.clone(),
                                discovered_images,
                            ) {
                                if let Some(position) = discovered_strings.iter().position(|s| s == current_image) {
                                    if queue.set_position(position) {
                                        self.queues.insert(output_name.to_string(), queue);
                                        self.timers.insert(output_name.to_string(), Instant::now());
                                        log::info!("Restored queue for {} with current image at position {}", 
                                            output_name, position);
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                Sorting::Ascending | Sorting::Descending => {
                    // Ordered mode: restore if image list unchanged
                    if discovered_strings == saved_state.images {
                        if let Some(mut queue) = Queue::new(
                            saved_state.queue_size,
                            saved_state.sorting.clone(),
                            discovered_images,
                        ) {
                            if queue.set_position(saved_state.queue_position) {
                                self.queues.insert(output_name.to_string(), queue);
                                self.timers.insert(output_name.to_string(), Instant::now());
                                log::info!("Restored queue for {} with current image at position {}", 
                                    output_name, saved_state.queue_position);
                                return true;
                            }
                        }
                    }
                }
            }
        }

        log::info!("Image list changed for {}, starting fresh", output_name);
        false
    }
    
    #[allow(dead_code)]
    fn get_group_for_output(&self, output_name: &str) -> Option<&MonitorGroup> {
        self.groups.iter().find(|group| group.outputs.contains(&output_name.to_string()))
    }
    
    #[allow(dead_code)]
    fn get_group_for_output_mut(&mut self, output_name: &str) -> Option<&mut MonitorGroup> {
        self.groups.iter_mut().find(|group| group.outputs.contains(&output_name.to_string()))
    }
    
    #[allow(dead_code)]
    fn find_outputs_in_same_group(&self, output_name: &str) -> Vec<String> {
        if let Some(group) = self.get_group_for_output(output_name) {
            group.outputs.clone()
        } else {
            vec![output_name.to_string()]
        }
    }
}

async fn initialize_output_queue(
    state: &mut DaemonState,
    output_name: &str,
    config: &Config,
) {
    let output_config = config.get_output_config(output_name);
    
    // Get image path from config, skip output if none specified
    let image_path = match &output_config.path {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            log::warn!("No wallpaper path configured for output '{}'", output_name);
            log::warn!("  Add a path to [any] section or create [outputs.\"{}\"] section in config", output_name);
            return;
        }
    };

    // Discover images
    let discovered_images = match ImageDiscovery::discover_images(&image_path) {
        Ok(images) => images,
        Err(e) => {
            log::error!("Failed to discover images for {}: {}", output_name, e.user_friendly_message());
            return;
        }
    };

    // Try to restore queue from state
    if !state.restore_queue_from_state(output_name, discovered_images.clone()) {
        // Create new queue if restoration failed
        if let Some(queue) = Queue::new(
            output_config.queue_size,
            output_config.sorting,
            discovered_images,
        ) {
            state.queues.insert(output_name.to_string(), queue);
            state.timers.insert(output_name.to_string(), Instant::now());
            
            // Set initial wallpaper if queue wasn't restored from state
            if let Some(current_image) = state.queues[output_name].current_image() {
                let command_builder = CommandBuilder::new(PathBuf::from("swww"));
                let executor = ProcessExecutor::new(command_builder);
                
                // Convert config to common format
                let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
                if let Err(e) = executor.execute_swww_command(
                    current_image,
                    &common_config,
                    Some(output_name),
                ).await {
                    log::error!("Failed to set initial wallpaper for {}: {}", 
                        output_name, e.user_friendly_message());
                } else {
                    log::info!("Set initial wallpaper for {}: {:?}", output_name, current_image);
                }
            }
        }
    }
}

fn reinitialize_daemon_state_sync(
    state: &mut DaemonState,
    config: &Config,
    swww_outputs: &[String],
) -> Result<(), anyhow::Error> {
    log::info!("Reinitializing daemon state (sync) due to configuration change...");
    
    // Clear existing state
    state.queues.clear();
    state.timers.clear();
    state.groups.clear();
    state.shared_queue = None;
    state.shared_timer = None;
    // Keep paused state
    
    // Reinitialize monitor behavior
    initialize_monitor_behavior(state, config, swww_outputs)?;
    
    // Reinitialize queues based on new behavior (using sync approaches)
    let behavior = config.get_effective_monitor_behavior();
    log::info!("Reinitializing (sync) with monitor behavior: {:?}", behavior);
    
    match behavior {
        MonitorBehavior::Independent => {
            log::info!("Reinitializing individual queues for Independent mode (sync)");
            for output_name in swww_outputs {
                initialize_output_queue_sync(state, output_name, config);
            }
        }
        MonitorBehavior::Synchronized => {
            log::info!("Reinitializing for Synchronized mode (sync)");
            if let Some(shared_queue) = &state.shared_queue {
                if let Some(current_image) = shared_queue.current_image() {
                    for output_name in swww_outputs {
                        let output_config = config.get_output_config(output_name);
                        let common_config = swwws_common::command_builder::OutputConfig {
                            path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
                            mode: None,
                            transition_type: Some(output_config.transition_type.clone()),
                            transition_step: Some(output_config.transition_step as u8),
                            transition_angle: Some(output_config.transition_angle),
                            transition_pos: Some(output_config.transition_pos.clone()),
                            transition_bezier: Some(output_config.transition_bezier.clone()),
                            transition_fps: None,
                            resize: Some(output_config.resize.clone()),
                            fill_color: Some(output_config.fill_color.clone()),
                            filter: Some(output_config.filter.clone()),
                            invert_y: Some(output_config.invert_y),
                            transition_wave: Some(output_config.transition_wave.clone()),
                        };
                        
                        set_wallpaper_sync(output_name, current_image, &common_config);
                    }
                }
            }
        }
        MonitorBehavior::Grouped(_) => {
            log::info!("Reinitializing for Grouped mode (sync)");
            for group in &state.groups {
                if let Some(current_image) = group.queue.current_image() {
                    for output_name in &group.outputs {
                        let output_config = config.get_output_config(output_name);
                        let common_config = swwws_common::command_builder::OutputConfig {
                            path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
                            mode: None,
                            transition_type: Some(output_config.transition_type.clone()),
                            transition_step: Some(output_config.transition_step as u8),
                            transition_angle: Some(output_config.transition_angle),
                            transition_pos: Some(output_config.transition_pos.clone()),
                            transition_bezier: Some(output_config.transition_bezier.clone()),
                            transition_fps: None,
                            resize: Some(output_config.resize.clone()),
                            fill_color: Some(output_config.fill_color.clone()),
                            filter: Some(output_config.filter.clone()),
                            invert_y: Some(output_config.invert_y),
                            transition_wave: Some(output_config.transition_wave.clone()),
                        };
                        
                        set_wallpaper_sync(output_name, current_image, &common_config);
                    }
                }
            }
            
            // Initialize independent queues for outputs not in any group
            for output_name in swww_outputs {
                if !state.groups.iter().any(|g| g.outputs.contains(output_name)) {
                    initialize_output_queue_sync(state, output_name, config);
                }
            }
        }
    }
    
    log::info!("State reinitialization (sync) complete: {} individual queues, {} groups, shared queue: {}", 
        state.queues.len(), state.groups.len(), state.shared_queue.is_some());
    
    Ok(())
}

fn initialize_output_queue_sync(
    state: &mut DaemonState,
    output_name: &str,
    config: &Config,
) {
    let output_config = config.get_output_config(output_name);
    
    // Get image path from config, skip output if none specified
    let image_path = match &output_config.path {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            log::warn!("No image path configured for output {}, skipping", output_name);
            return;
        }
    };

    // Discover images
    let discovered_images = match ImageDiscovery::discover_images(&image_path) {
        Ok(images) => images,
        Err(e) => {
            log::error!("Failed to discover images for {}: {}", output_name, e.user_friendly_message());
            return;
        }
    };

    // Try to restore queue from state or create new one
    if !state.restore_queue_from_state(output_name, discovered_images.clone()) {
        if let Some(queue) = Queue::new(
            output_config.queue_size,
            output_config.sorting,
            discovered_images,
        ) {
            state.queues.insert(output_name.to_string(), queue);
            state.timers.insert(output_name.to_string(), Instant::now());
            
            // Set initial wallpaper if queue wasn't restored from state
            if let Some(current_image) = state.queues[output_name].current_image() {
                let common_config = swwws_common::command_builder::OutputConfig {
                    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
                    mode: None,
                    transition_type: Some(output_config.transition_type.clone()),
                    transition_step: Some(output_config.transition_step as u8),
                    transition_angle: Some(output_config.transition_angle),
                    transition_pos: Some(output_config.transition_pos.clone()),
                    transition_bezier: Some(output_config.transition_bezier.clone()),
                    transition_fps: None,
                    resize: Some(output_config.resize.clone()),
                    fill_color: Some(output_config.fill_color.clone()),
                    filter: Some(output_config.filter.clone()),
                    invert_y: Some(output_config.invert_y),
                    transition_wave: Some(output_config.transition_wave.clone()),
                };
                
                set_wallpaper_sync(output_name, current_image, &common_config);
            }
        }
    }
}

fn set_wallpaper_sync(
    output_name: &str,
    image_path: &PathBuf,
    common_config: &swwws_common::command_builder::OutputConfig,
) {
    let command_builder = CommandBuilder::new(PathBuf::from("swww"));
    let executor = ProcessExecutor::new(command_builder);
    let output_name_clone = output_name.to_string();
    let image_path_clone = image_path.clone();
    let common_config_clone = common_config.clone();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(e) = executor.execute_swww_command(
                &image_path_clone,
                &common_config_clone,
                Some(&output_name_clone),
            ).await {
                log::error!("Failed to set wallpaper for {}: {}", 
                    output_name_clone, e.user_friendly_message());
            } else {
                log::info!("Set wallpaper for {}: {:?}", output_name_clone, image_path_clone);
            }
        });
    });
}

fn initialize_monitor_behavior(
    state: &mut DaemonState,
    config: &Config,
    swww_outputs: &[String],
) -> Result<(), anyhow::Error> {
    match config.get_effective_monitor_behavior() {
        MonitorBehavior::Independent => {
            log::info!("Using independent monitor behavior - each output manages its own queue");
            // Nothing special to initialize - each output has its own queue
        }
        MonitorBehavior::Synchronized => {
            log::info!("Using synchronized monitor behavior - all outputs share the same queue");
            // Create a shared queue using the first available path
            let first_output = swww_outputs.first()
                .ok_or_else(|| anyhow::anyhow!("No display outputs available for synchronized mode"))?;
            let output_config = config.get_output_config(first_output);
            let image_path = output_config.path.as_ref()
                .ok_or_else(|| {
                    anyhow::anyhow!("No wallpaper path configured for synchronized mode. Add 'path = \"/path/to/wallpapers\"' to [any] section in config")
                })?;
            
            let discovered_images = ImageDiscovery::discover_images(&PathBuf::from(image_path))
                .map_err(|e| anyhow::anyhow!("Failed to discover images for synchronized mode: {}", e.user_friendly_message()))?;
            
            if let Some(shared_queue) = Queue::new(
                output_config.queue_size,
                output_config.sorting,
                discovered_images,
            ) {
                state.shared_queue = Some(shared_queue);
                state.shared_timer = Some(Instant::now());
                log::info!("Created shared queue for synchronized mode with {} images", 
                    state.shared_queue.as_ref().unwrap().size());
            }
        }
        MonitorBehavior::Grouped(groups) => {
            log::info!("Using grouped monitor behavior with {} groups", groups.len());
            
            for (group_idx, group_outputs) in groups.iter().enumerate() {
                let group_name = format!("group_{}", group_idx);
                log::info!("Initializing group '{}' with outputs: {:?}", group_name, group_outputs);
                
                // Find the first output in this group that has a path configured
                let mut group_path = None;
                let mut group_config = None;
                
                for output in group_outputs {
                    if swww_outputs.contains(output) {
                        let output_config = config.get_output_config(output);
                        if output_config.path.is_some() {
                            group_path = output_config.path.clone();
                            group_config = Some(output_config);
                            break;
                        }
                    }
                }
                
                if let (Some(path), Some(config_data)) = (group_path, group_config) {
                    let discovered_images = ImageDiscovery::discover_images(&PathBuf::from(&path))
                        .map_err(|e| anyhow::anyhow!("Failed to discover images for group '{}': {}", group_name, e.user_friendly_message()))?;
                    
                    if let Some(queue) = Queue::new(
                        config_data.queue_size,
                        config_data.sorting,
                        discovered_images,
                    ) {
                        let monitor_group = MonitorGroup {
                            name: group_name.clone(),
                            outputs: group_outputs.iter()
                                .filter(|output| swww_outputs.contains(output))
                                .map(|s| s.to_string())
                                .collect(),
                            queue,
                            timer: Instant::now(),
                        };
                        
                        log::info!("Created group '{}' with {} outputs and {} images", 
                            group_name, monitor_group.outputs.len(), monitor_group.queue.size());
                        state.groups.push(monitor_group);
                    }
                } else {
                    log::warn!("Group '{}' has no valid outputs with configured paths, skipping", group_name);
                }
            }
        }
    }
    
    Ok(())
}

async fn change_wallpaper(
    output_name: &str,
    image_path: &std::path::Path,
    config: &Config,
    executor: &ProcessExecutor,
) {
    let output_config = config.get_output_config(output_name);
    
    // Convert config to common format
    let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };

    // Execute swww command with retry logic
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_millis(500);
    
    for attempt in 0..MAX_RETRIES {
        match executor.execute_swww_command(
            image_path,
            &common_config,
            Some(output_name),
        ).await {
            Ok(()) => {
                log::info!("Set wallpaper for {}: {:?}", output_name, image_path);
                return;
            }
            Err(e) => {
                if attempt < MAX_RETRIES - 1 {
                    log::warn!("Failed to set wallpaper for {} (attempt {}/{}): {}. Retrying in {}ms...", 
                        output_name, attempt + 1, MAX_RETRIES, e.user_friendly_message(), 
                        RETRY_DELAY.as_millis());
                    tokio::time::sleep(RETRY_DELAY).await;
                } else {
                    log::error!("Failed to set wallpaper for {} after {} attempts: {}", 
                        output_name, MAX_RETRIES, e.user_friendly_message());
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    log::info!("Starting swwws daemon...");

    // Load configuration
    let config = Config::load()
        .map_err(|e| {
            log::error!("Configuration error: {}", e.user_friendly_message());
            anyhow::anyhow!("Configuration error: {}", e.user_friendly_message())
        })?;

    log::info!("Configuration loaded successfully");

    // Check if swww daemon is running
    ProcessExecutor::check_swww_daemon()
        .map_err(|e| {
            log::error!("swww daemon check failed: {}", e.user_friendly_message());
            anyhow::anyhow!("swww daemon check failed: {}", e.user_friendly_message())
        })?;

    // Get swww outputs
    let swww_outputs = ProcessExecutor::get_swww_outputs()
        .map_err(|e| {
            log::error!("Failed to get swww outputs: {}", e.user_friendly_message());
            anyhow::anyhow!("Failed to get swww outputs: {}", e.user_friendly_message())
        })?;

    if swww_outputs.is_empty() {
        return Err(anyhow::anyhow!("No swww outputs found"));
    }

    log::info!("Found swww outputs: {:?}", swww_outputs);

    // Initialize daemon state
    let mut state = DaemonState::new()
        .map_err(|e| {
            log::error!("Failed to initialize daemon state: {}", e);
            anyhow::anyhow!("Failed to initialize daemon state: {}", e)
        })?;

    // Initialize monitor behavior (groups, synchronized, etc.)
    if let Err(e) = initialize_monitor_behavior(&mut state, &config, &swww_outputs) {
        log::error!("Failed to initialize monitor behavior: {}", e);
        return Err(e);
    }

    // Initialize individual queues based on monitor behavior
    let behavior = config.get_effective_monitor_behavior();
    log::info!("Detected monitor behavior: {:?}", behavior);
    match behavior {
        MonitorBehavior::Independent => {
            // Initialize queues for each output independently
            log::info!("Initializing individual queues for Independent mode");
            for output_name in &swww_outputs {
                initialize_output_queue(&mut state, output_name, &config).await;
            }
        }
        MonitorBehavior::Synchronized => {
            // For synchronized mode, set initial wallpaper on all outputs from shared queue
            log::info!("Setting initial synchronized wallpapers (no individual queues)");
            if let Some(shared_queue) = &state.shared_queue {
                if let Some(current_image) = shared_queue.current_image() {
                    let command_builder = CommandBuilder::new(PathBuf::from("swww"));
                    let executor = ProcessExecutor::new(command_builder);
                    
                    for output_name in &swww_outputs {
                        let output_config = config.get_output_config(output_name);
                        let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
                        
                        if let Err(e) = executor.execute_swww_command(
                            current_image,
                            &common_config,
                            Some(output_name),
                        ).await {
                            log::error!("Failed to set initial synchronized wallpaper for {}: {}", 
                                output_name, e.user_friendly_message());
                        } else {
                            log::info!("Set initial synchronized wallpaper for {}: {:?}", output_name, current_image);
                        }
                    }
                }
            } else {
                log::error!("Synchronized mode enabled but no shared queue created!");
            }
        }
        MonitorBehavior::Grouped(_) => {
            // For grouped mode, set initial wallpaper for each group
            let command_builder = CommandBuilder::new(PathBuf::from("swww"));
            let executor = ProcessExecutor::new(command_builder);
            
            for group in &state.groups {
                if let Some(current_image) = group.queue.current_image() {
                    for output_name in &group.outputs {
                        let output_config = config.get_output_config(output_name);
                        let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
                        
                        if let Err(e) = executor.execute_swww_command(
                            current_image,
                            &common_config,
                            Some(output_name),
                        ).await {
                            log::error!("Failed to set initial group wallpaper for {}: {}", 
                                output_name, e.user_friendly_message());
                        } else {
                            log::info!("Set initial group wallpaper for {}: {:?}", output_name, current_image);
                        }
                    }
                }
            }
            
            // Also initialize independent queues for outputs not in any group
            for output_name in &swww_outputs {
                if !state.groups.iter().any(|g| g.outputs.contains(output_name)) {
                    initialize_output_queue(&mut state, output_name, &config).await;
                }
            }
        }
    }

    // Validate that we have at least one way to manage wallpapers
    let has_individual_queues = !state.queues.is_empty();
    let has_shared_queue = state.shared_queue.is_some();
    let has_groups = !state.groups.is_empty();
    
    if !has_individual_queues && !has_shared_queue && !has_groups {
        let behavior_name = match config.get_effective_monitor_behavior() {
            MonitorBehavior::Independent => "Independent",
            MonitorBehavior::Synchronized => "Synchronized",
            MonitorBehavior::Grouped(_) => "Grouped",
        };
        
        log::error!("Failed to initialize wallpaper management for {} monitor behavior", behavior_name);
        log::error!("Possible causes:");
        log::error!("  - No wallpaper paths configured in config file");
        log::error!("  - Configured paths don't exist or contain no valid images");
        log::error!("  - Display outputs don't match configuration (found: {:?})", swww_outputs);
        
        // Give specific hints based on monitor behavior
        match config.get_effective_monitor_behavior() {
            MonitorBehavior::Independent => {
                log::error!("  - For Independent mode: configure [any] path or specific [outputs.\"OUTPUT-NAME\"] sections");
            },
            MonitorBehavior::Synchronized => {
                log::error!("  - For Synchronized mode: ensure [any] section has a valid 'path' setting");
            },
            MonitorBehavior::Grouped(_) => {
                log::error!("  - For Grouped mode: ensure monitor_groups are configured with valid paths");
            },
        }
        
        return Err(anyhow::anyhow!("No valid wallpaper management initialized - check configuration and paths"));
    }
    
    log::info!("Daemon initialized successfully: {} individual queues, {} groups, shared queue: {}", 
        state.queues.len(), state.groups.len(), has_shared_queue);

    // Create shared state for IPC
    let shared_state = Arc::new(Mutex::new(state));
    let command_builder = CommandBuilder::new(PathBuf::from("swww"));
    let executor = ProcessExecutor::new(command_builder);

    // Start IPC server
    let ipc_state = Arc::clone(&shared_state);
    let ipc_executor = executor.clone();
    
    std::thread::spawn(move || {
        let server = IpcServer::new();
        if let Err(e) = server.start(move |cmd| {
            Ok(handle_ipc_command(cmd, Arc::clone(&ipc_state), ipc_executor.clone()))
        }) {
            log::error!("IPC server error: {}", e);
        }
    });

    log::info!("Daemon started successfully with {} outputs", shared_state.lock().unwrap().queues.len());

    // Main timer loop with error recovery
    let mut interval = interval(Duration::from_secs(1));
    let mut save_counter = 0;
    let mut swww_check_counter = 0;

    loop {
        interval.tick().await;
        save_counter += 1;
        swww_check_counter += 1;

        // Periodically check if swww daemon is still running (every 30 seconds)
        if swww_check_counter >= 30 {
            swww_check_counter = 0;
            match ProcessExecutor::check_swww_daemon() {
                Ok(()) => {
                    // swww daemon is running, all good
                }
                Err(e) => {
                    log::error!("swww daemon check failed: {}. Attempting to recover...", e.user_friendly_message());
                    // Wait a bit and try again
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    match ProcessExecutor::check_swww_daemon() {
                        Ok(()) => {
                            log::info!("swww daemon recovered successfully");
                        }
                        Err(e2) => {
                            log::error!("swww daemon still not available after retry: {}. Continuing to monitor...", e2.user_friendly_message());
                            // Don't exit, just keep trying - user might restart swww daemon
                            continue;
                        }
                    }
                }
            }
        }

        let mut state_guard = match shared_state.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                log::warn!("Failed to acquire state lock, skipping this cycle");
                continue;
            }
        };

        // Skip processing if paused
        if state_guard.paused {
            continue;
        }

        // Check for expired timers
        let mut expired_outputs = Vec::new();
        for (output_name, timer) in &state_guard.timers {
            let output_config = config.get_output_config(output_name);
            let target_duration: Duration = output_config.duration;
            
            if timer.elapsed() >= target_duration {
                expired_outputs.push(output_name.clone());
            }
        }

        // Process timers based on monitor behavior
        let behavior = config.get_effective_monitor_behavior();
        
        match behavior {
            MonitorBehavior::Independent => {
                // Process individual output timers
                if !expired_outputs.is_empty() {
                    for output_name in expired_outputs {
                        if let Some(queue) = state_guard.queues.get_mut(&output_name) {
                            if let Some(next_image) = queue.next() {
                                change_wallpaper(&output_name, &next_image, &config, &executor).await;
                                state_guard.timers.insert(output_name.clone(), Instant::now());
                            }
                        }
                    }
                }
            }
            MonitorBehavior::Synchronized => {
                // Check shared timer
                if let Some(shared_timer) = &state_guard.shared_timer {
                    let target_duration = config.get_output_config(&swww_outputs[0]).duration;
                    if shared_timer.elapsed() >= target_duration {
                        if let Some(shared_queue) = &mut state_guard.shared_queue {
                            if let Some(next_image) = shared_queue.next() {
                                log::info!("Synchronized mode: Setting same image on all outputs: {:?}", next_image);
                                // Set the same image on all outputs
                                for output_name in &swww_outputs {
                                    change_wallpaper(output_name, &next_image, &config, &executor).await;
                                }
                                state_guard.shared_timer = Some(Instant::now());
                            }
                        }
                    }
                }
            }
            MonitorBehavior::Grouped(_) => {
                // Check group timers
                for group in &mut state_guard.groups {
                    let target_duration = if let Some(first_output) = group.outputs.first() {
                        config.get_output_config(first_output).duration
                    } else {
                        Duration::from_secs(300) // fallback
                    };
                    
                    if group.timer.elapsed() >= target_duration {
                        if let Some(next_image) = group.queue.next() {
                            log::info!("Group '{}': Setting image on grouped outputs: {:?}", group.name, next_image);
                            // Set the same image on all outputs in this group
                            for output_name in &group.outputs {
                                change_wallpaper(output_name, &next_image, &config, &executor).await;
                            }
                            group.timer = Instant::now();
                        }
                    }
                }
                
                // Also process individual output timers for outputs not in any group
                if !expired_outputs.is_empty() {
                    for output_name in expired_outputs {
                        // Only process if output is not in any group
                        let is_in_group = state_guard.groups.iter().any(|g| g.outputs.contains(&output_name));
                        if !is_in_group {
                            if let Some(queue) = state_guard.queues.get_mut(&output_name) {
                                if let Some(next_image) = queue.next() {
                                    change_wallpaper(&output_name, &next_image, &config, &executor).await;
                                    state_guard.timers.insert(output_name.clone(), Instant::now());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Save state periodically (every 30 seconds)
        if save_counter >= 30 {
            if let Err(e) = state_guard.save_state() {
                log::error!("Failed to save state: {}", e);
            }
            save_counter = 0;
        }
    }
}

fn handle_next_for_output(
    state: &mut DaemonState,
    output_name: &str,
    config: &Config,
    executor: &ProcessExecutor,
) {
    if let Some(queue) = state.queues.get_mut(output_name) {
        if let Some(next_image) = queue.next() {
            change_wallpaper_sync(output_name, &next_image, config, executor);
            state.timers.insert(output_name.to_string(), Instant::now());
        }
    }
}

fn handle_previous_for_output(
    state: &mut DaemonState,
    output_name: &str,
    config: &Config,
    executor: &ProcessExecutor,
) {
    if let Some(queue) = state.queues.get_mut(output_name) {
        if let Some(prev_image) = queue.previous() {
            let output_config = config.get_output_config(output_name);
            let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
            execute_wallpaper_change(output_name, &prev_image, &common_config, executor);
            state.timers.insert(output_name.to_string(), Instant::now());
        }
    }
}

fn execute_wallpaper_change(
    output_name: &str,
    image_path: &PathBuf,
    common_config: &swwws_common::command_builder::OutputConfig,
    executor: &ProcessExecutor,
) {
    let executor_clone = executor.clone();
    let output_name_clone = output_name.to_string();
    let image_path_clone = image_path.clone();
    let common_config_clone = common_config.clone();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(e) = executor_clone.execute_swww_command(
                &image_path_clone,
                &common_config_clone,
                Some(&output_name_clone),
            ).await {
                log::error!("Failed to set wallpaper for {}: {}", 
                    output_name_clone, e.user_friendly_message());
            } else {
                log::info!("Set wallpaper for {}: {:?}", output_name_clone, image_path_clone);
            }
        });
    });
}

fn change_wallpaper_sync(
    output_name: &str,
    image_path: &std::path::Path,
    config: &Config,
    executor: &ProcessExecutor,
) {
    let output_config = config.get_output_config(output_name);
    
    let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };

    let executor_clone = executor.clone();
    let output_name_clone = output_name.to_string();
    let image_path_clone = image_path.to_path_buf();

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(e) = executor_clone.execute_swww_command(
                &image_path_clone,
                &common_config,
                Some(&output_name_clone),
            ).await {
                log::error!("Failed to set wallpaper for {}: {}", 
                    output_name_clone, e.user_friendly_message());
            } else {
                log::info!("Set wallpaper for {}: {:?}", output_name_clone, image_path_clone);
            }
        });
    });
}

fn handle_ipc_command(
    command: IpcCommand,
    state: Arc<Mutex<DaemonState>>,
    executor: ProcessExecutor,
) -> IpcResponse {
    let mut state_guard = state.lock().unwrap();
    
    // Load config to check monitor behavior
    let config = match swwws_config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            return IpcResponse::Error { 
                message: format!("Failed to load config: {}", e.user_friendly_message()) 
            };
        }
    };

    match command {
        IpcCommand::Next { output } => {
            if let Some(specific_output) = output {
                // Specific output requested - ignore monitor behavior
                handle_next_for_output(&mut state_guard, &specific_output, &config, &executor);
            } else {
                // Handle based on current daemon state (not config, which might be out of sync)
                let current_behavior = if state_guard.shared_queue.is_some() {
                    MonitorBehavior::Synchronized
                } else if !state_guard.groups.is_empty() {
                    let groups: Vec<Vec<String>> = state_guard.groups.iter()
                        .map(|group| group.outputs.clone())
                        .collect();
                    MonitorBehavior::Grouped(groups)
                } else {
                    MonitorBehavior::Independent
                };
                match current_behavior {
                    MonitorBehavior::Independent => {
                        // Each output advances independently
                        let outputs: Vec<_> = state_guard.queues.keys().cloned().collect();
                        for output_name in outputs {
                            handle_next_for_output(&mut state_guard, &output_name, &config, &executor);
                        }
                    }
                    MonitorBehavior::Synchronized => {
                        // All outputs show the same next image from shared queue
                        let next_image = if let Some(shared_queue) = &mut state_guard.shared_queue {
                            shared_queue.next().cloned()
                        } else {
                            None
                        };
                        
                        if let Some(image_path) = next_image {
                            log::info!("IPC Synchronized: Setting same image {:?} on all outputs", image_path);
                            // Get all available outputs
                            let swww_outputs = match ProcessExecutor::get_swww_outputs() {
                                Ok(outputs) => outputs,
                                Err(_) => {
                                    // Fallback: collect queue keys without borrowing state_guard mutably
                                    vec![] // We'll handle this case below
                                }
                            };
                            
                            let outputs_to_use = if swww_outputs.is_empty() {
                                // Only collect if we need to, and do it separately
                                state_guard.queues.keys().cloned().collect::<Vec<_>>()
                            } else {
                                swww_outputs
                            };
                            
                            for output_name in &outputs_to_use {
                                change_wallpaper_sync(output_name, &image_path, &config, &executor);
                            }
                            state_guard.shared_timer = Some(Instant::now());
                        }
                    }
                    MonitorBehavior::Grouped(_) => {
                        // Advance all groups and independent outputs
                        for group in &mut state_guard.groups {
                            if let Some(next_image) = group.queue.next() {
                                log::info!("IPC Group '{}': Setting image {:?} on group outputs", group.name, next_image);
                                for output_name in &group.outputs {
                                    change_wallpaper_sync(output_name, &next_image, &config, &executor);
                                }
                                group.timer = Instant::now();
                            }
                        }
                        
                        // Also advance independent outputs not in any group
                        let outputs: Vec<_> = state_guard.queues.keys().cloned().collect();
                        for output_name in outputs {
                            let is_in_group = state_guard.groups.iter().any(|g| g.outputs.contains(&output_name));
                            if !is_in_group {
                                handle_next_for_output(&mut state_guard, &output_name, &config, &executor);
                            }
                        }
                    }
                }
            }

            IpcResponse::Success { message: "Next wallpaper set".to_string() }
        }

        IpcCommand::Previous { output } => {
            if let Some(specific_output) = output {
                // Handle specific output request
                handle_previous_for_output(&mut state_guard, &specific_output, &config, &executor);
            } else {
                // Handle based on current daemon state (not config, which might be out of sync)
                let current_behavior = if state_guard.shared_queue.is_some() {
                    MonitorBehavior::Synchronized
                } else if !state_guard.groups.is_empty() {
                    let groups: Vec<Vec<String>> = state_guard.groups.iter()
                        .map(|group| group.outputs.clone())
                        .collect();
                    MonitorBehavior::Grouped(groups)
                } else {
                    MonitorBehavior::Independent
                };
                match current_behavior {
                    MonitorBehavior::Independent => {
                        let outputs: Vec<_> = state_guard.queues.keys().cloned().collect();
                        for output_name in outputs {
                            handle_previous_for_output(&mut state_guard, &output_name, &config, &executor);
                        }
                    }
                    MonitorBehavior::Synchronized => {
                        if let Some(shared_queue) = &mut state_guard.shared_queue {
                            if let Some(prev_image) = shared_queue.previous() {
                                log::info!("IPC Synchronized: Setting previous image {:?} on all outputs", prev_image);
                                let swww_outputs = match ProcessExecutor::get_swww_outputs() {
                                    Ok(outputs) => outputs,
                                    Err(_) => vec![] // fallback
                                };
                                for output_name in &swww_outputs {
                                    let output_config = config.get_output_config(output_name);
                                    let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
                                    execute_wallpaper_change(output_name, &prev_image, &common_config, &executor);
                                }
                                state_guard.shared_timer = Some(Instant::now());
                            }
                        }
                    }
                    MonitorBehavior::Grouped(_) => {
                        // Handle groups
                        for group in &mut state_guard.groups {
                            if let Some(prev_image) = group.queue.previous() {
                                log::info!("IPC Group '{}': Setting previous image {:?} on group outputs", group.name, prev_image);
                                for output_name in &group.outputs {
                                    let output_config = config.get_output_config(output_name);
                                    let common_config = swwws_common::command_builder::OutputConfig {
    path: output_config.path.as_ref().map(|p| PathBuf::from(p)),
    mode: None,
    transition_type: Some(output_config.transition_type.clone()),
    transition_step: Some(output_config.transition_step as u8),
    transition_angle: Some(output_config.transition_angle),
    transition_pos: Some(output_config.transition_pos.clone()),
    transition_bezier: Some(output_config.transition_bezier.clone()),
    transition_fps: None,
    resize: Some(output_config.resize.clone()),
    fill_color: Some(output_config.fill_color.clone()),
    filter: Some(output_config.filter.clone()),
    invert_y: Some(output_config.invert_y),
    transition_wave: Some(output_config.transition_wave.clone()),
                };
                                    execute_wallpaper_change(output_name, &prev_image, &common_config, &executor);
                                }
                                group.timer = Instant::now();
                            }
                        }
                        
                        // Handle independent outputs
                        let outputs: Vec<_> = state_guard.queues.keys().cloned().collect();
                        for output_name in outputs {
                            let is_in_group = state_guard.groups.iter().any(|g| g.outputs.contains(&output_name));
                            if !is_in_group {
                                handle_previous_for_output(&mut state_guard, &output_name, &config, &executor);
                            }
                        }
                    }
                }
            }
            
            IpcResponse::Success { message: "Previous wallpaper set".to_string() }
        }

        IpcCommand::Pause => {
            state_guard.paused = true;
            IpcResponse::Success { message: "Slideshow paused".to_string() }
        }

        IpcCommand::Resume => {
            state_guard.paused = false;
            IpcResponse::Success { message: "Slideshow resumed".to_string() }
        }

        IpcCommand::TogglePause => {
            state_guard.paused = !state_guard.paused;
            let status = if state_guard.paused { "paused" } else { "resumed" };
            IpcResponse::Success { message: format!("Slideshow {}", status) }
        }

        IpcCommand::Reload => {
            // Reload configuration with comprehensive error handling
            match swwws_config::Config::load() {
                Ok(new_config) => {
                    // Validate new config before applying
                    match new_config.get_effective_monitor_behavior() {
                        swwws_common::MonitorBehavior::Grouped(ref groups) if groups.is_empty() => {
                            let error_msg = "Invalid config: grouped behavior with empty groups";
                            log::error!("{}", error_msg);
                            return IpcResponse::Error { message: error_msg.to_string() };
                        }
                        _ => {}
                    }
                    
                    // Check if swww daemon is still accessible with new config
                    if let Err(e) = ProcessExecutor::check_swww_daemon() {
                        let error_msg = format!("Cannot reload: swww daemon not accessible: {}", e.user_friendly_message());
                        log::error!("{}", error_msg);
                        return IpcResponse::Error { message: error_msg };
                    }
                    
                    // Try to get outputs to ensure they're still valid
                    let swww_outputs = match ProcessExecutor::get_swww_outputs() {
                        Ok(outputs) => {
                            if outputs.is_empty() {
                                let error_msg = "Cannot reload: no swww outputs available";
                                log::error!("{}", error_msg);
                                return IpcResponse::Error { message: error_msg.to_string() };
                            }
                            outputs
                        }
                        Err(e) => {
                            let error_msg = format!("Cannot reload: failed to get swww outputs: {}", e.user_friendly_message());
                            log::error!("{}", error_msg);
                            return IpcResponse::Error { message: error_msg };
                        }
                    };
                    
                    // Check if monitor behavior has changed by inferring current behavior from daemon state
                    let current_behavior = if state_guard.shared_queue.is_some() {
                        MonitorBehavior::Synchronized
                    } else if !state_guard.groups.is_empty() {
                        // For grouped mode, we need to reconstruct the groups structure
                        let groups: Vec<Vec<String>> = state_guard.groups.iter()
                            .map(|group| group.outputs.clone())
                            .collect();
                        MonitorBehavior::Grouped(groups)
                    } else {
                        MonitorBehavior::Independent
                    };
                    let new_behavior = new_config.get_effective_monitor_behavior();
                    
                    if std::mem::discriminant(&current_behavior) != std::mem::discriminant(&new_behavior) {
                        log::info!("Monitor behavior changed from {:?} to {:?}, reinitializing daemon state", 
                            current_behavior, new_behavior);
                        
                        // Reinitialize state with new behavior (using sync version)
                        if let Err(e) = reinitialize_daemon_state_sync(&mut state_guard, &new_config, &swww_outputs) {
                            let error_msg = format!("Failed to reinitialize daemon state: {}", e);
                            log::error!("{}", error_msg);
                            return IpcResponse::Error { message: error_msg };
                        }
                        
                        log::info!("Daemon state reinitialized successfully for new monitor behavior");
                        IpcResponse::Success { message: "Configuration reloaded and daemon state reinitialized for new monitor behavior".to_string() }
                    } else {
                        // Same monitor behavior, just validate and update queues if needed
                        log::info!("Monitor behavior unchanged, configuration reloaded successfully");
                        IpcResponse::Success { message: "Configuration reloaded successfully".to_string() }
                    }
                }
                Err(e) => {
                    log::error!("Failed to reload configuration: {}", e.user_friendly_message());
                    IpcResponse::Error { message: format!("Failed to reload configuration: {}", e.user_friendly_message()) }
                }
            }
        }

        IpcCommand::Status => {
            let mut statuses = Vec::new();
            // Use daemon state to determine current behavior, not config
            let behavior = if state_guard.shared_queue.is_some() {
                MonitorBehavior::Synchronized
            } else if !state_guard.groups.is_empty() {
                let groups: Vec<Vec<String>> = state_guard.groups.iter()
                    .map(|group| group.outputs.clone())
                    .collect();
                MonitorBehavior::Grouped(groups)
            } else {
                MonitorBehavior::Independent
            };
            log::debug!("Status command - detected behavior: {:?}", behavior);
            log::debug!("Status command - individual queues count: {}", state_guard.queues.len());
            log::debug!("Status command - has shared queue: {}", state_guard.shared_queue.is_some());
            
            match behavior {
                MonitorBehavior::Independent => {
                    // Show individual queue status for each output
                    for (output_name, queue) in &state_guard.queues {
                        let timer = state_guard.timers.get(output_name);
                        let elapsed = timer.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
                        let output_config = config.get_output_config(output_name);
                        let target_duration = output_config.duration;
                        let remaining = if elapsed >= target_duration {
                            Duration::ZERO
                        } else {
                            target_duration - elapsed
                        };

                        let current_image = queue.current_image()
                            .map(|p| p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().to_string());

                        statuses.push(OutputStatus {
                            name: output_name.clone(),
                            current_image,
                            queue_position: queue.current_position(),
                            queue_size: queue.size(),
                            timer_remaining: Some(remaining.as_secs()),
                            paused: state_guard.paused,
                        });
                    }
                }
                MonitorBehavior::Synchronized => {
                    // Show synchronized status for all outputs
                    let swww_outputs = ProcessExecutor::get_swww_outputs().unwrap_or_default();
                    if let Some(shared_queue) = &state_guard.shared_queue {
                        let timer = &state_guard.shared_timer;
                        let elapsed = timer.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
                        let target_duration = if let Some(first_output) = swww_outputs.first() {
                            config.get_output_config(first_output).duration
                        } else {
                            Duration::from_secs(300)
                        };
                        let remaining = if elapsed >= target_duration {
                            Duration::ZERO
                        } else {
                            target_duration - elapsed
                        };

                        let current_image = shared_queue.current_image()
                            .map(|p| p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().to_string());

                        // Add status for all outputs showing they're synchronized
                        for output_name in swww_outputs {
                            statuses.push(OutputStatus {
                                name: format!("{} (sync)", output_name),
                                current_image: current_image.clone(),
                                queue_position: shared_queue.current_position(),
                                queue_size: shared_queue.size(),
                                timer_remaining: Some(remaining.as_secs()),
                                paused: state_guard.paused,
                            });
                        }
                    }
                }
                MonitorBehavior::Grouped(_) => {
                    // Show group status
                    for group in &state_guard.groups {
                        let elapsed = group.timer.elapsed();
                        let target_duration = if let Some(first_output) = group.outputs.first() {
                            config.get_output_config(first_output).duration
                        } else {
                            Duration::from_secs(300)
                        };
                        let remaining = if elapsed >= target_duration {
                            Duration::ZERO
                        } else {
                            target_duration - elapsed
                        };

                        let current_image = group.queue.current_image()
                            .map(|p| p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().to_string());

                        // Add status for all outputs in this group
                        for output_name in &group.outputs {
                            statuses.push(OutputStatus {
                                name: format!("{} ({})", output_name, group.name),
                                current_image: current_image.clone(),
                                queue_position: group.queue.current_position(),
                                queue_size: group.queue.size(),
                                timer_remaining: Some(remaining.as_secs()),
                                paused: state_guard.paused,
                            });
                        }
                    }
                    
                    // Also show independent outputs not in any group
                    for (output_name, queue) in &state_guard.queues {
                        let is_in_group = state_guard.groups.iter().any(|g| g.outputs.contains(output_name));
                        if !is_in_group {
                            let timer = state_guard.timers.get(output_name);
                            let elapsed = timer.map(|t| t.elapsed()).unwrap_or(Duration::ZERO);
                            let output_config = config.get_output_config(output_name);
                            let target_duration = output_config.duration;
                            let remaining = if elapsed >= target_duration {
                                Duration::ZERO
                            } else {
                                target_duration - elapsed
                            };

                            let current_image = queue.current_image()
                                .map(|p| p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().to_string());

                            statuses.push(OutputStatus {
                                name: format!("{} (independent)", output_name),
                                current_image,
                                queue_position: queue.current_position(),
                                queue_size: queue.size(),
                                timer_remaining: Some(remaining.as_secs()),
                                paused: state_guard.paused,
                            });
                        }
                    }
                }
            }

            IpcResponse::Status { outputs: statuses, paused: state_guard.paused }
        }
    }
}
