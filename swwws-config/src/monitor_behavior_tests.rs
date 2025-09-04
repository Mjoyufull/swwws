#[cfg(test)]
mod tests {
    use crate::*;
    use swwws_common::MonitorBehavior;

    #[test]
    fn test_default_monitor_behavior() {
        let config_content = r#"
            [any]
            path = "/test/path"
        "#;

        let config: Config = toml::from_str(config_content).expect("Config should parse");
        assert_eq!(config.monitor_behavior, MonitorBehavior::Independent);
        assert_eq!(config.get_effective_monitor_behavior(), MonitorBehavior::Independent);
    }

    #[test]
    fn test_explicit_independent_monitor_behavior() {
        let config_content = r#"
            monitor_behavior = "Independent"
            
            [any]
            path = "/test/path"
        "#;

        let config: Config = toml::from_str(config_content).expect("Config should parse");
        assert_eq!(config.monitor_behavior, MonitorBehavior::Independent);
        assert_eq!(config.get_effective_monitor_behavior(), MonitorBehavior::Independent);
    }

    #[test]
    fn test_synchronized_monitor_behavior() {
        let config_content = r#"
            monitor_behavior = "Synchronized"
            
            [any]
            path = "/test/path"
        "#;

        let config: Config = toml::from_str(config_content).expect("Config should parse");
        assert_eq!(config.monitor_behavior, MonitorBehavior::Synchronized);
        assert_eq!(config.get_effective_monitor_behavior(), MonitorBehavior::Synchronized);
    }

    #[test]
    fn test_monitor_groups_validation() {
        // Test that monitor_groups field can be set independently
        let config_content = r#"
            monitor_behavior = "Independent"
            monitor_groups = [
                ["HDMI-A-1", "DP-2"],
                ["DP-3"]
            ]
            
            [any]
            path = "/test/path"
        "#;

        let config: Config = toml::from_str(config_content).expect("Config should parse");
        config.validate().expect("Config should validate");
        
        assert_eq!(config.monitor_behavior, MonitorBehavior::Independent);
        assert!(config.monitor_groups.is_some());
        let groups = config.monitor_groups.as_ref().unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["HDMI-A-1", "DP-2"]);
        assert_eq!(groups[1], vec!["DP-3"]);
    }

    #[test]
    fn test_invalid_monitor_behavior_fails() {
        let config_content = r#"
            monitor_behavior = "InvalidMode"
            
            [any]
            path = "/test/path"
        "#;

        let result: std::result::Result<Config, _> = toml::from_str(config_content);
        assert!(result.is_err(), "Invalid monitor behavior should fail to parse");
    }
}
