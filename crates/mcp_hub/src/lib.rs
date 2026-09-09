use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpGrant {
    pub resource_id: String,
    pub granted_scope: String,
}

pub struct McpHubServer {
    pub grants: Vec<McpGrant>,
}

impl McpHubServer {
    pub fn new() -> Self {
        Self { grants: Vec::new() }
    }

    pub fn add_grant(&mut self, grant: McpGrant) {
        self.grants.push(grant);
    }

    pub fn has_grant(&self, resource_id: &str, scope: &str) -> bool {
        self.grants.iter().any(|g| g.resource_id == resource_id && g.granted_scope == scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_hub_grants() {
        let mut hub = McpHubServer::new();
        hub.add_grant(McpGrant { resource_id: "res1".into(), granted_scope: "read".into() });
        assert!(hub.has_grant("res1", "read"));
        assert!(!hub.has_grant("res1", "write"));
    }
}
