use std::path::Path;
use std::process::Command;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub path: Option<PathBuf>,
    pub mode: Option<String>,
    pub transition_type: Option<String>,
    pub transition_step: Option<u8>,
    pub transition_angle: Option<f32>,
    pub transition_pos: Option<String>,
    pub transition_bezier: Option<String>,
    pub transition_fps: Option<u8>,
    pub resize: Option<String>,
    pub fill_color: Option<String>,
    pub filter: Option<String>,
    pub invert_y: Option<bool>,
    pub transition_wave: Option<String>,
}

#[derive(Clone)]
pub struct CommandBuilder {
    swww_path: PathBuf,
}

impl CommandBuilder {
    pub fn new(swww_path: PathBuf) -> Self {
        Self { swww_path }
    }

    pub fn build_img_command(
        &self,
        image_path: &Path,
        config: &OutputConfig,
        output_name: Option<&str>,
    ) -> Command {
        let mut cmd = Command::new(&self.swww_path);
        cmd.arg("img");

        if let Some(output) = output_name {
            cmd.args(&["-o", output]);
        }
        if let Some(transition_type) = &config.transition_type {
            cmd.args(&["--transition-type", transition_type]);
        }
        if let Some(transition_step) = config.transition_step {
            cmd.args(&["--transition-step", &transition_step.to_string()]);
        }
        if let Some(transition_angle) = config.transition_angle {
            cmd.args(&["--transition-angle", &transition_angle.to_string()]);
        }
        if let Some(transition_pos) = &config.transition_pos {
            cmd.args(&["--transition-pos", transition_pos]);
        }
        if let Some(transition_bezier) = &config.transition_bezier {
            cmd.args(&["--transition-bezier", transition_bezier]);
        }
        if let Some(transition_fps) = config.transition_fps {
            cmd.args(&["--transition-fps", &transition_fps.to_string()]);
        }
        if let Some(resize) = &config.resize {
            cmd.args(&["--resize", resize]);
        }
        if let Some(fill_color) = &config.fill_color {
            cmd.args(&["--fill-color", fill_color]);
        }
        if let Some(filter) = &config.filter {
            cmd.args(&["-f", filter]);
        }
        if let Some(invert_y) = config.invert_y {
            if invert_y {
                cmd.arg("--invert-y");
            }
        }
        if let Some(transition_wave) = &config.transition_wave {
            cmd.args(&["--transition-wave", transition_wave]);
        }
        cmd.arg(image_path);
        cmd
    }
}
