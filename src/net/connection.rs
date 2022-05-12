#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
#[serde(default)]
pub struct ConnectionData {
    server: String,
    name: String,
}

impl Default for ConnectionData {
    fn default() -> Self {
        Self {
            server: "127.0.0.1:6078".to_string(),
            name: "nobody".to_string(),
        }
    }
}

impl ConnectionData {
    pub fn new(server: &str, name: &str) -> Self {
        Self {
            server: server.to_string(),
            name: name.to_string(),
        }
    }

    pub fn server(&self) -> &String {
        &self.server
    }

    pub fn name(&self) -> &String {
        &self.name
    }
}
