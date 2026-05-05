use std::fmt;
use std::str::FromStr;

/// Service groups that can be enabled in the MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceGroup {
    Market,
    Account,
    Trade,
    Funding,
    Paper,
    Auth,
}

impl ServiceGroup {
    /// All known service groups.
    pub fn all() -> Vec<ServiceGroup> {
        vec![
            ServiceGroup::Market,
            ServiceGroup::Account,
            ServiceGroup::Trade,
            ServiceGroup::Funding,
            ServiceGroup::Paper,
            ServiceGroup::Auth,
        ]
    }

    /// Default service groups (safe operations, no auth required for some).
    pub fn default_groups() -> Vec<ServiceGroup> {
        vec![ServiceGroup::Market, ServiceGroup::Account, ServiceGroup::Paper]
    }

    /// Whether this group contains dangerous operations (trade, funding).
    pub fn is_dangerous(&self) -> bool {
        matches!(self, ServiceGroup::Trade | ServiceGroup::Funding)
    }

    /// Parse a comma-separated list of service group names.
    /// Valid names: market, account, trade, funding, paper, auth
    /// "all" expands to all groups.
    pub fn parse(s: &str) -> Result<Vec<ServiceGroup>, String> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("all") {
            return Ok(Self::all());
        }

        let mut groups = Vec::new();
        for part in trimmed.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            match part.to_ascii_lowercase().as_str() {
                "market" => groups.push(ServiceGroup::Market),
                "account" => groups.push(ServiceGroup::Account),
                "trade" => groups.push(ServiceGroup::Trade),
                "funding" => groups.push(ServiceGroup::Funding),
                "paper" => groups.push(ServiceGroup::Paper),
                "auth" => groups.push(ServiceGroup::Auth),
                _ => return Err(format!("Unknown service group: '{}'", part)),
            }
        }

        if groups.is_empty() {
            return Err("No service groups specified".into());
        }

        Ok(groups)
    }
}

impl fmt::Display for ServiceGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceGroup::Market => write!(f, "market"),
            ServiceGroup::Account => write!(f, "account"),
            ServiceGroup::Trade => write!(f, "trade"),
            ServiceGroup::Funding => write!(f, "funding"),
            ServiceGroup::Paper => write!(f, "paper"),
            ServiceGroup::Auth => write!(f, "auth"),
        }
    }
}

impl FromStr for ServiceGroup {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "market" => Ok(ServiceGroup::Market),
            "account" => Ok(ServiceGroup::Account),
            "trade" => Ok(ServiceGroup::Trade),
            "funding" => Ok(ServiceGroup::Funding),
            "paper" => Ok(ServiceGroup::Paper),
            "auth" => Ok(ServiceGroup::Auth),
            _ => Err(format!("Unknown service group: '{}'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = ServiceGroup::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_single() {
        let groups = ServiceGroup::parse("market").unwrap();
        assert_eq!(groups, vec![ServiceGroup::Market]);
    }

    #[test]
    fn test_parse_multiple() {
        let groups = ServiceGroup::parse("market,trade,paper").unwrap();
        assert_eq!(
            groups,
            vec![ServiceGroup::Market, ServiceGroup::Trade, ServiceGroup::Paper]
        );
    }

    #[test]
    fn test_parse_all() {
        let groups = ServiceGroup::parse("all").unwrap();
        assert_eq!(groups.len(), 6);
        assert!(groups.contains(&ServiceGroup::Market));
        assert!(groups.contains(&ServiceGroup::Funding));
    }

    #[test]
    fn test_parse_case_insensitive() {
        let groups = ServiceGroup::parse("Market,TRADE").unwrap();
        assert_eq!(groups, vec![ServiceGroup::Market, ServiceGroup::Trade]);
    }

    #[test]
    fn test_parse_unknown_group() {
        let result = ServiceGroup::parse("market,unknown");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown"));
    }

    #[test]
    fn test_parse_with_spaces() {
        let groups = ServiceGroup::parse(" market , paper ").unwrap();
        assert_eq!(groups, vec![ServiceGroup::Market, ServiceGroup::Paper]);
    }

    #[test]
    fn test_default_groups() {
        let groups = ServiceGroup::default_groups();
        assert_eq!(groups.len(), 3);
        assert!(groups.contains(&ServiceGroup::Market));
        assert!(groups.contains(&ServiceGroup::Account));
        assert!(groups.contains(&ServiceGroup::Paper));
    }

    #[test]
    fn test_is_dangerous() {
        assert!(!ServiceGroup::Market.is_dangerous());
        assert!(!ServiceGroup::Account.is_dangerous());
        assert!(ServiceGroup::Trade.is_dangerous());
        assert!(ServiceGroup::Funding.is_dangerous());
        assert!(!ServiceGroup::Paper.is_dangerous());
        assert!(!ServiceGroup::Auth.is_dangerous());
    }

    #[test]
    fn test_display() {
        assert_eq!(ServiceGroup::Market.to_string(), "market");
        assert_eq!(ServiceGroup::Trade.to_string(), "trade");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("market".parse::<ServiceGroup>().unwrap(), ServiceGroup::Market);
        assert_eq!("TRADE".parse::<ServiceGroup>().unwrap(), ServiceGroup::Trade);
        assert!("invalid".parse::<ServiceGroup>().is_err());
    }

    #[test]
    fn test_all_contains_all() {
        let all = ServiceGroup::all();
        assert_eq!(all.len(), 6);
        for group in &[ServiceGroup::Market, ServiceGroup::Account, ServiceGroup::Trade,
                       ServiceGroup::Funding, ServiceGroup::Paper, ServiceGroup::Auth] {
            assert!(all.contains(group));
        }
    }
}
