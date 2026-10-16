use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct PolicyHandler {
    policy: HashMap<String, Component>,
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
    pub fn new(policy_file_path: String) -> Self {
        let toml_content =
            fs::read_to_string(policy_file_path).expect("Failed to read config.toml");

        let policy_handler: PolicyHandler =
            toml::from_str(&toml_content).expect("Failed to parse config.toml");

        policy_handler
    }

    pub fn get_policy(&self) -> Vec<Component> {
        self.policy.values().cloned().collect()
    }

    pub fn show_policy(&self) {
        self.policy.iter().for_each(|(_, field)| {
            println!("|-----------------");
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
