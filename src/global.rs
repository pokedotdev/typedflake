use crate::Config;
use std::sync::OnceLock;
use thiserror::Error;

/// Unified global defaults containing both config and instance
#[derive(Debug, Clone, Copy, Default)]
struct GlobalDefaults {
    config: Config,
    instance: (u64, u64),
}

/// Global defaults state
static GLOBAL_DEFAULTS: OnceLock<GlobalDefaults> = OnceLock::new();

/// Error types for default configuration
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DefaultConfigError {
    #[error("Global defaults have already been set")]
    DefaultsAlreadySet,
}

/// Set both default configuration and instance (can only be called once)
pub fn set_defaults(
    config: Config,
    worker_id: u64,
    process_id: u64,
) -> Result<(), DefaultConfigError> {
    let defaults = GlobalDefaults {
        config,
        instance: (worker_id, process_id),
    };
    GLOBAL_DEFAULTS
        .set(defaults)
        .map_err(|_| DefaultConfigError::DefaultsAlreadySet)
}

/// Set the default configuration (can only be called once)
pub fn set_default_config(config: Config) -> Result<(), DefaultConfigError> {
    let defaults = GlobalDefaults {
        config,
        instance: (0, 0),
    };
    GLOBAL_DEFAULTS
        .set(defaults)
        .map_err(|_| DefaultConfigError::DefaultsAlreadySet)
}

/// Set the default instance (worker_id, process_id) - can only be called once
pub fn set_default_instance(worker_id: u64, process_id: u64) -> Result<(), DefaultConfigError> {
    let defaults = GlobalDefaults {
        config: Config::default(),
        instance: (worker_id, process_id),
    };
    GLOBAL_DEFAULTS
        .set(defaults)
        .map_err(|_| DefaultConfigError::DefaultsAlreadySet)
}

/// Get the default configuration, initializing with hardcoded default if not set
pub fn get_default_config() -> Config {
    GLOBAL_DEFAULTS.get_or_init(GlobalDefaults::default).config
}

/// Get the default instance, initializing with (0, 0) if not set
pub fn get_default_instance() -> (u64, u64) {
    GLOBAL_DEFAULTS
        .get_or_init(GlobalDefaults::default)
        .instance
}

/// Check if global defaults have been set
pub fn is_defaults_set() -> bool {
    GLOBAL_DEFAULTS.get().is_some()
}

/// Check if default configuration has been set
pub fn is_default_config_set() -> bool {
    GLOBAL_DEFAULTS.get().is_some()
}

/// Check if default instance has been set
pub fn is_default_instance_set() -> bool {
    GLOBAL_DEFAULTS.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_initialization() {
        // Before setting anything, should return default config
        let config = get_default_config();
        let default_config = Config::default();

        assert_eq!(config.timestamp_bits, default_config.timestamp_bits);
        assert_eq!(config.worker_bits, default_config.worker_bits);
        assert_eq!(config.process_bits, default_config.process_bits);
        assert_eq!(config.sequence_bits, default_config.sequence_bits);
        assert_eq!(config.epoch_ms, default_config.epoch_ms);
    }

    #[test]
    fn test_default_instance_initialization() {
        // Before setting anything, should return (0, 0)
        let (worker_id, process_id) = get_default_instance();
        assert_eq!(worker_id, 0);
        assert_eq!(process_id, 0);
    }

    // Note: We cannot test the actual setting functionality in unit tests
    // because OnceLock can only be set once per program execution.
    // These will be tested in integration tests where each test gets
    // a fresh program instance.
}
