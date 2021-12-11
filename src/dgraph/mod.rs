use reqwest::Response;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::CanariaError;

mod dal;

pub type NodeID = String; // Hex encoded u64, i.e. "0x4a5b"


// MARK: Query result structs

#[allow(dead_code)]
#[derive(Deserialize)]
struct ResultExtensions {
    /// `{parsing,processing,encoding,assign_timestamp,total}_ns`
    server_latency: HashMap<String, u32>,
    /// Transaction stats: `start_ts`
    txn: HashMap<String, u32>,
    /// Result metrics per queried predicate plus `_total` field count
    metrics: HashMap<String, HashMap<String, u32>>
}


#[allow(dead_code)]
#[derive(Deserialize)]
struct ResultError {
    message: String,
    extensions: HashMap<String, String>
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ResultData<T> {
    /// Results grouped by query name
    data: HashMap<String, Vec<T>>,
 
    #[serde(default = "Vec::new")]
    errors: Vec<ResultError>,
    
    /// Dgraph query metadata
    extensions: ResultExtensions,
}


// MARK: Client

/// Way to interact with DGraph backend
pub struct DgraphClient {
    base_url: String,
}

impl DgraphClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    // MARK: Data-strict manipulation methods

    /// Drop all database data
    pub async fn drop_all(&self) -> Result<(), CanariaError> {
        let mut map = HashMap::new();
        map.insert("drop_all", true);
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/alter");
        let res: Result<Response, CanariaError> = client
            .post(url.as_str())
            .json(&map)
            .send()
            .await
            .map_err(|e| e.into());
        let log = res?.text().await?;
        log::debug!("dropping database data: {:?}", log);
        Ok(())
    }

    /// (re)Defines dgraph internal (DQL) schema
    pub async fn set_schema(&self, schemafile: &Path) -> Result<(), CanariaError> {
        let schema = std::fs::read_to_string(schemafile)?;
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/alter");
        let res: Result<Response, CanariaError> = client
            .post(url.as_str())
            .body(schema)
            .send()
            .await
            .map_err(|e| e.into());
        let log = res?.text().await?;
        log::debug!("setting schema: {:?}", log);
        Ok(())
    }

    /// Executes a mutation resulting only success or error
    pub async fn mutate(&self, rdf: &str) -> Result<(), CanariaError> {
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/mutate?commitNow=true");
        log::debug!("DQL Mutation: \n{}", format!("{{ set {{ {} }} }}", rdf));
        let res: Result<Response, CanariaError> = client
            .post(url.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/rdf")
            .body(format!("{{ set {{ {} }} }}", rdf))
            .send()
            .await
            .map_err(|e| e.into());
        let log = res?.text().await?;
        log::debug!("mutation response: {:?}", log);
        Ok(())
    }

    /// Read-only query returning an array of values
    pub async fn query<T>(&self, dql: &str) -> Result<Vec<T>, CanariaError> 
    where T: for<'de> Deserialize<'de> + Clone {
        if dql.is_empty() {
            return Err("empty query string".into());
        }

        let q_name = dql.split("(").next();
        if let None = q_name {
            return Err("could not get query name".into());
        }
        let q_name = q_name.unwrap().trim();

        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/query");
        log::debug!("DQL Query:\n{}", format!("{{ {} }}", dql));
        let res: Result<Response, CanariaError> = client
            .post(url.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/dql")
            .body(format!("{{ {} }}", dql))
            .send()
            .await
            .map_err(|e| e.into());
        let r_data = res?.json::<ResultData<T>>().await?;
        let result = r_data.data.get(q_name).ok_or("no data matching query name")?;
        Ok(result.to_vec())
    }
    
    /// Read-only query returning a single element
    pub async fn query_single<T>(&self, dql: &str) -> Result<Option<T>, CanariaError>
    where T: for<'de> Deserialize<'de> + Clone {
        let mut results = self.query(dql).await?;
        if results.is_empty() {
            return Ok(None)
        }
        Ok(results.swap_remove(0))
    }
}
