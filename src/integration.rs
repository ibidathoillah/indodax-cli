//! Stable integration surface for consumers that embed this crate as a
//! submodule, path dependency, git dependency, or SDK-style library.

pub use crate::client::IndodaxClient;
pub use crate::config::{IndodaxConfig, ResolvedCredentials, SecretValue};
pub use crate::errors::IndodaxError;
#[cfg(feature = "cli")]
pub use crate::output::{CommandOutput, OutputFormat};

/// SDK-focused imports for application integrations.
///
/// This module intentionally avoids CLI command dispatch types, so desktop
/// apps and services can embed `indodax-cli` without coupling to the CLI API.
pub mod sdk {
    pub use crate::client::IndodaxClient;
    pub use crate::config::{IndodaxConfig, ResolvedCredentials, SecretValue};
    pub use crate::errors::IndodaxError;

    /// Recommended imports for SDK consumers.
    pub mod prelude {
        pub use super::{
            IndodaxClient, IndodaxConfig, IndodaxError, ResolvedCredentials, SecretValue,
        };
    }
}

/// Recommended imports for application integrations.
pub mod prelude {
    pub use super::{IndodaxClient, IndodaxConfig, IndodaxError, ResolvedCredentials, SecretValue};
}

/// CLI-specific integration API.
///
/// Use this only when the consumer needs to embed command dispatching.
#[cfg(all(feature = "cli", not(target_arch = "wasm32")))]
pub mod cli {
    pub use crate::output::{CommandOutput, OutputFormat};
    pub use crate::{dispatch, map_anyhow_error, Cli, Command};
}
