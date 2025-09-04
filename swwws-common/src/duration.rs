use std::time::Duration;
use std::str::FromStr;
use anyhow::{Result, Context};

pub fn parse_duration(duration_str: &str) -> Result<Duration> {
    humantime::Duration::from_str(duration_str)
        .map(|d| d.into())
        .with_context(|| format!("Invalid duration format: {}", duration_str))
}
