//! Global default configuration and instance management.
//!
//! Provides one-time initialization of default configuration and (worker_id, process_id)
//! for all ID types in the application. Useful for distributed systems where each service
//! instance has the same configuration throughout its lifecycle.
//!
//! Use [`defaults()`] at application startup before generating any IDs:
//!
//! ```rust,no_run
//! typedflake_core::global::defaults().instance(1, 2).init().unwrap();
//! ```

use crate::Config;
use derive_more::{Display, Error};
use std::sync::OnceLock;

/// Unified global defaults containing both config and instance
#[derive(Debug, Clone, Copy, Default)]
struct GlobalDefaults {
    config: Config,
    instance: (u64, u64),
}

/// Global defaults state
static GLOBAL_DEFAULTS: OnceLock<GlobalDefaults> = OnceLock::new();

/// Error types for default configuration
#[derive(Display, Error, Debug, Clone, PartialEq, Eq)]
pub enum DefaultConfigError {
    #[display("Global defaults have already been set")]
    DefaultsAlreadySet,
}

/// Builder for configuring global defaults.
///
/// Created via [`defaults()`]. Call [`init()`](DefaultsBuilder::init) to apply.
pub struct DefaultsBuilder {
    config: Option<Config>,
    instance: Option<(u64, u64)>,
}

impl DefaultsBuilder {
    /// Set the global default configuration.
    pub fn config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the global default worker and process IDs.
    pub fn instance(mut self, worker_id: u64, process_id: u64) -> Self {
        self.instance = Some((worker_id, process_id));
        self
    }

    /// Initialize global defaults. Unset fields use defaults (`Config::DEFAULT`, `(0, 0)`).
    ///
    /// Can only be called once. Returns an error if defaults have already been set.
    pub fn init(self) -> Result<(), DefaultConfigError> {
        let defaults = GlobalDefaults {
            config: self.config.unwrap_or_default(),
            instance: self.instance.unwrap_or((0, 0)),
        };
        GLOBAL_DEFAULTS
            .set(defaults)
            .map_err(|_| DefaultConfigError::DefaultsAlreadySet)
    }
}

/// Create a builder to configure global defaults.
///
/// # Example
///
/// ```rust,no_run
/// use typedflake_core::global::defaults;
///
/// defaults().instance(1, 2).init().unwrap();
/// ```
pub fn defaults() -> DefaultsBuilder {
    DefaultsBuilder {
        config: None,
        instance: None,
    }
}

/// Get default config (initializes if not set)
pub fn get_default_config() -> Config {
    GLOBAL_DEFAULTS.get_or_init(GlobalDefaults::default).config
}

/// Get default instance (initializes to (0,0) if not set)
pub fn get_default_instance() -> (u64, u64) {
    GLOBAL_DEFAULTS
        .get_or_init(GlobalDefaults::default)
        .instance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_fallback() {
        // Before setting anything, should return default config
        let config = get_default_config();
        let default_config = Config::default();

        assert_eq!(
            config.layout().timestamp(),
            default_config.layout().timestamp()
        );
        assert_eq!(config.layout().worker(), default_config.layout().worker());
        assert_eq!(config.layout().process(), default_config.layout().process());
        assert_eq!(
            config.layout().sequence(),
            default_config.layout().sequence()
        );
        assert_eq!(config.epoch(), default_config.epoch());
    }

    #[test]
    fn default_instance_zero_zero() {
        // Unconfigured default should return (0, 0)
        let (worker_id, process_id) = get_default_instance();
        assert_eq!(worker_id, 0);
        assert_eq!(process_id, 0);
    }

    // Note: We cannot test the actual setting functionality in unit tests
    // because OnceLock can only be set once per program execution.
    // These will be tested in integration tests where each test gets
    // a fresh program instance.
}
