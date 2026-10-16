use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Loads policy files describing the ship components and the NMEA traffic they may send/receive.
#[derive(Debug, Deserialize)]
pub struct PolicyHandler {
    /// `BTreeMap` so components are returned in a deterministic (TOML key) order.
    policy: BTreeMap<String, Component>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Component {
    pub name: String,
    pub iface: String,
    pub mac: String,
    pub ip: String,
    pub sends: Vec<String>,
    pub receives: Vec<String>,
}

impl PolicyHandler {
    /// Reads and parses the policy file at `policy_file_path`.
    pub fn new(policy_file_path: impl AsRef<Path>) -> Result<Self> {
        let path = policy_file_path.as_ref();

        let toml_content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read policy file '{}'", path.display()))?;

        let policy_handler: PolicyHandler = toml::from_str(&toml_content)
            .with_context(|| format!("failed to parse policy file '{}'", path.display()))?;

        Ok(policy_handler)
    }

    /// Returns the policy entries in deterministic (TOML key) order.
    pub fn get_policy(&self) -> Vec<Component> {
        self.policy.values().cloned().collect()
    }

    pub fn show_policy(&self) {
        self.policy.iter().for_each(|(key, field)| {
            println!("|-----------------");
            println!("| Key: {}", key);
            println!("| Name: {}", field.name);
            println!("| Iface: {}", field.iface);
            println!("| MAC: {}", field.mac);
            println!("| IP: {}", field.ip);
            println!("| Sends: {:?}", field.sends);
            println!("| Receives: {:?}", field.receives);
            println!("|-----------------");
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn unique_dir(test_name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("afxdp-policy-{}-{}", std::process::id(), test_name));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_policy(dir: &Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("policy.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn parses_valid_policy_file() {
        let dir = unique_dir("valid");
        let path = write_policy(
            &dir,
            r#"
[policy]
c0 = { name = "girobussola", iface = "test1", mac = "54:00:00:00:00:10", ip = "10.42.0.10", sends = [
  "$IIHDT",
], receives = [] }
c1 = { name = "radar", iface = "test6", mac = "54:00:00:00:00:60", ip = "10.42.0.60", sends = [], receives = [
  "$IIHDT",
] }
"#,
        );

        let handler = PolicyHandler::new(&path).unwrap();
        let policy = handler.get_policy();

        assert_eq!(policy.len(), 2);
        assert_eq!(policy[0].name, "girobussola");
        assert_eq!(policy[0].sends, vec!["$IIHDT"]);
        assert_eq!(policy[1].name, "radar");
        assert_eq!(policy[1].receives, vec!["$IIHDT"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn returns_components_in_key_order() {
        let dir = unique_dir("order");
        let path = write_policy(
            &dir,
            r#"
[policy]
c1 = { name = "second", iface = "test2", mac = "54:00:00:00:00:20", ip = "10.42.0.20", sends = [], receives = [] }
c0 = { name = "first", iface = "test1", mac = "54:00:00:00:00:10", ip = "10.42.0.10", sends = [], receives = [] }
"#,
        );

        let policy = PolicyHandler::new(&path).unwrap().get_policy();
        assert_eq!(policy[0].name, "first");
        assert_eq!(policy[1].name, "second");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_is_an_error() {
        assert!(PolicyHandler::new("/nonexistent/policy.toml").is_err());
    }

    #[test]
    fn malformed_toml_is_an_error() {
        let dir = unique_dir("malformed");
        let path = write_policy(&dir, "this is not valid toml :::");

        assert!(PolicyHandler::new(&path).is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
