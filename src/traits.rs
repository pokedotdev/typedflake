/// Automatically implement standard traits and methods for ID types
#[macro_export]
macro_rules! impl_id_traits {
    ($name:ident) => {
        impl $name {
            /// Create an ID from a raw u64 value without validation.
            pub fn from_u64_unchecked(id: u64) -> Self {
                Self(id)
            }

            /// Create an ID from a raw u64 value with validation.
            pub fn try_from_u64(id: u64) -> Result<Self, $crate::config::ValidationError> {
                Self::context().config().validate_id(id)?;
                Ok(Self(id))
            }

            /// Get the raw u64 value of this ID
            pub fn as_u64(self) -> u64 {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::convert::TryFrom<u64> for $name {
            type Error = $crate::config::ValidationError;

            fn try_from(id: u64) -> Result<Self, Self::Error> {
                Self::try_from_u64(id)
            }
        }

        impl From<$name> for u64 {
            fn from(id: $name) -> u64 {
                id.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::config::ValidationError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let id = s.parse::<u64>()?;
                Self::try_from_u64(id)
            }
        }
    };
}
