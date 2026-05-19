//! Stable integration surface for consumers that embed this crate as a
//! submodule or path dependency.

pub use crate::client::IndodaxClient;
pub use crate::config::{IndodaxConfig, ResolvedCredentials, SecretValue};
pub use crate::errors::IndodaxError;
pub use crate::output::{CommandOutput, OutputFormat};
pub use crate::{dispatch, map_anyhow_error, Cli, Command};

/// Convenience imports for external consumers.
pub mod prelude {
    pub use super::{
        dispatch, map_anyhow_error, Cli, Command, CommandOutput, IndodaxConfig, IndodaxError,
        IndodaxClient, OutputFormat, ResolvedCredentials, SecretValue,
    };
}
