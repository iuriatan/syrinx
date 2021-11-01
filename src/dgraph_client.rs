use reqwest::{Response, Error};
use std::collections::HashMap;
use std::path::Path;

use crate::CanariaError;

pub struct DgraphClient {
    dsn: String
}

impl DgraphClient {
    pub fn new(dsn: String) -> Self {
        Self{ dsn }
    }

    pub async fn drop_all(&self) -> Result<Response, CanariaError> {
        let mut map = HashMap::new();
        map.insert("drop_all", "true");
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.dsn, "/alter");
        client.post(url.as_str())
            .json(&map)
            .send()
            .await
            .map_err(|e| e.into())
    }
    
    pub async fn set_schema(&self, schemafile: &Path) -> Result<Response, CanariaError> {
        let schema = std::fs::read_to_string(schemafile)?;
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.dsn, "/alter");
        client.post(url.as_str())
            .body(schema)
            .send()
            .await
            .map_err(|e| e.into())
    }
}