use super::service::ServiceGroup;

/// Configuration for safety/guarding of dangerous MCP operations.
#[derive(Debug, Clone)]
pub struct SafetyConfig {
    /// If true, dangerous operations (trade, funding) are allowed without
    /// the `acknowledged` parameter being true.
    pub allow_dangerous: bool,
}

impl SafetyConfig {
    /// Create a new safety configuration.
    ///
    /// `allow_dangerous` should be true only when the user explicitly opts in
    /// (e.g., via `--allow-dangerous` CLI flag).
    pub fn new(allow_dangerous: bool) -> Self {
        Self { allow_dangerous }
    }

    /// Check whether an operation in the given service group is allowed.
    ///
    /// Returns `Ok(())` if allowed, or `Err(message)` with guidance.
    pub fn check_group(&self, group: &ServiceGroup) -> Result<(), String> {
        if group.is_dangerous() && !self.allow_dangerous {
            return Err(format!(
                "[DANGEROUS: requires acknowledged=true] The '{}' service group contains dangerous operations. \
                 Provide `acknowledged: true` in your tool call arguments, or start the MCP server with \
                 `--allow-dangerous` to disable this guard.",
                group
            ));
        }
        Ok(())
    }

    /// Check whether a dangerous operation is allowed with the given acknowledged flag.
    ///
    /// Must provide `acknowledged: true` for dangerous groups unless `allow_dangerous` is set.
    pub fn check_operation(&self, group: &ServiceGroup, acknowledged: bool) -> Result<(), String> {
        if group.is_dangerous() && !self.allow_dangerous && !acknowledged {
            return Err(format!(
                "[DANGEROUS: requires acknowledged=true] This operation belongs to the '{}' group. \
                 Set `acknowledged` to `true` to confirm you want to proceed, or start the MCP server \
                 with `--allow-dangerous` to disable this check entirely.",
                group
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_group_allowed_with_guarded() {
        let config = SafetyConfig::new(false);
        assert!(config.check_group(&ServiceGroup::Market).is_ok());
        assert!(config.check_group(&ServiceGroup::Account).is_ok());
        assert!(config.check_group(&ServiceGroup::Paper).is_ok());
        assert!(config.check_group(&ServiceGroup::Auth).is_ok());
        assert!(config.check_group(&ServiceGroup::Alert).is_ok());
    }

    #[test]
    fn test_dangerous_group_blocked_with_guarded() {
        let config = SafetyConfig::new(false);
        assert!(config.check_group(&ServiceGroup::Trade).is_err());
        assert!(config.check_group(&ServiceGroup::Funding).is_err());
    }

    #[test]
    fn test_dangerous_group_allowed_with_allow_dangerous() {
        let config = SafetyConfig::new(true);
        assert!(config.check_group(&ServiceGroup::Trade).is_ok());
        assert!(config.check_group(&ServiceGroup::Funding).is_ok());
    }

    #[test]
    fn test_operation_blocked_without_acknowledged() {
        let config = SafetyConfig::new(false);
        assert!(config.check_operation(&ServiceGroup::Trade, false).is_err());
        assert!(config
            .check_operation(&ServiceGroup::Funding, false)
            .is_err());
    }

    #[test]
    fn test_operation_allowed_with_acknowledged() {
        let config = SafetyConfig::new(false);
        assert!(config.check_operation(&ServiceGroup::Trade, true).is_ok());
        assert!(config.check_operation(&ServiceGroup::Funding, true).is_ok());
    }

    #[test]
    fn test_safe_operation_no_acknowledgment_needed() {
        let config = SafetyConfig::new(false);
        assert!(config.check_operation(&ServiceGroup::Market, false).is_ok());
        assert!(config
            .check_operation(&ServiceGroup::Account, false)
            .is_ok());
        assert!(config.check_operation(&ServiceGroup::Paper, false).is_ok());
    }

    #[test]
    fn test_allow_dangerous_overrides_acknowledged() {
        let config = SafetyConfig::new(true);
        // Even without acknowledged, dangerous ops are allowed
        assert!(config.check_operation(&ServiceGroup::Trade, false).is_ok());
        assert!(config
            .check_operation(&ServiceGroup::Funding, false)
            .is_ok());
    }

    #[test]
    fn test_error_message_mentions_acknowledged() {
        let config = SafetyConfig::new(false);
        let err = config
            .check_operation(&ServiceGroup::Trade, false)
            .unwrap_err();
        assert!(err.contains("acknowledged"));
        assert!(err.contains("true"));
    }
}
