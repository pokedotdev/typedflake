use crate::Config;
use std::sync::OnceLock;
use thiserror::Error;

/// Default configuration state
static DEFAULT_CONFIG: OnceLock<Config> = OnceLock::new();
static DEFAULT_INSTANCE: OnceLock<(u64, u64)> = OnceLock::new();

/// Error types for default configuration
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum DefaultConfigError {
    #[error("Default configuration has already been set")]
    ConfigAlreadySet,
    #[error("Default instance has already been set")]
    InstanceAlreadySet,
}

/// Set the default configuration (can only be called once)
pub fn set_default_config(config: Config) -> Result<(), DefaultConfigError> {
    DEFAULT_CONFIG
        .set(config)
        .map_err(|_| DefaultConfigError::ConfigAlreadySet)
}

/// Set the default instance (worker_id, process_id) - can only be called once
pub fn set_default_instance(worker_id: u64, process_id: u64) -> Result<(), DefaultConfigError> {
    DEFAULT_INSTANCE
        .set((worker_id, process_id))
        .map_err(|_| DefaultConfigError::InstanceAlreadySet)
}

/// Get the default configuration, initializing with hardcoded default if not set
pub fn get_default_config() -> Config {
    *DEFAULT_CONFIG.get_or_init(Config::default)
}

/// Get the default instance, initializing with (0, 0) if not set
pub fn get_default_instance() -> (u64, u64) {
    *DEFAULT_INSTANCE.get_or_init(|| (0, 0))
}

/// Check if default configuration has been set
pub fn is_default_config_set() -> bool {
    DEFAULT_CONFIG.get().is_some()
}

/// Check if default instance has been set
pub fn is_default_instance_set() -> bool {
    DEFAULT_INSTANCE.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_initialization() {
        // Before setting anything, should return default config
        let config = get_default_config();
        let default_config = Config::default();

        assert_eq!(config.bits.timestamp, default_config.bits.timestamp);
        assert_eq!(config.bits.worker, default_config.bits.worker);
        assert_eq!(config.bits.process, default_config.bits.process);
        assert_eq!(config.bits.sequence, default_config.bits.sequence);
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
