#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemHealth {
    pub status: String,
    pub version: String,
    pub storage_ok: bool,
}

pub fn check_system_health() -> SystemHealth {
    SystemHealth {
        status: "OK".to_string(),
        version: "0.1.0".to_string(),
        storage_ok: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_health() {
        let health = check_system_health();
        assert_eq!(health.status, "OK");
        assert!(health.storage_ok);
    }
}
