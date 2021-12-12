use regex::Regex;
use reqwest::Response;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::CanariaError;

mod dal;

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
    #[serde(default = "HashMap::new")]
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
    pub async fn mutate(&self, dql: &str) -> Result<(), CanariaError> {
        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/mutate?commitNow=true");
        log::debug!("DQL Mutation: \n{}", dql);
        let res: Result<Response, CanariaError> = client
            .post(url.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/rdf")
            .body(dql.to_owned())
            .send()
            .await
            .map_err(|e| e.into());
        let log = res?.text().await?;
        log::debug!("mutation response: {:?}", log);
        Ok(())
    }

    /// Read-only query returning an "raw" database response
    async fn query(&self, dql: String) -> Result<Response, CanariaError> {
        if dql.is_empty() {
            return Err("empty query string".into());
        }

        let client = reqwest::Client::new();
        let url = format!("{}{}", self.base_url, "/query");
        log::debug!("DQL Query:\n{}", dql);
        client
            .post(url.as_str())
            .header(reqwest::header::CONTENT_TYPE, "application/dql")
            .body(dql)
            .send()
            .await
            .map_err(|e| e.into())
    }
    
    /// Read-only query returning a single element
    pub async fn query_single<T>(&self, dql: &str) -> Result<Option<T>, CanariaError>
    where T: for<'de> Deserialize<'de> + Clone {
        let q_names = extract_query_names(dql);
        if q_names.len() != 1 {
            return match q_names.len() {
                0 => Err("could not extract query names from DQL".into()),
                _ => Err("multiple query not supported".into())
            }
        }

        let db_result = self.query(dql.into()).await?;
        let mut results: ResultData<T> = db_result.json().await?;
        let mut result = results.data.remove(&q_names[0]).unwrap();
        if result.is_empty() {
            return Ok(None)
        }
        Ok(Some(result.swap_remove(0)))
    }
}

pub trait RDFable {
    fn nqd<S: std::fmt::Display>(&self, subject: S, predicate: S) -> String;
}

impl<T> RDFable for Option<T> where T: std::fmt::Display {
    fn nqd<S: std::fmt::Display>(&self, subject: S, predicate: S) -> String {
        match self {
            Some(object) => format!("{} {} \"{}\" .\n", subject, predicate, object),
            None => "".into()
        }
    }
}

// This shall be unified with `impl for Vec<PathBuf> for a generic type when PathBuf
// implements Display or Into<String>
impl RDFable for Vec<String> {
    fn nqd<S: std::fmt::Display>(&self, subject: S, predicate: S) -> String {
        let mut out: String = "".into();
        for object in self {
            out = format!("{}{} {} \"{}\" .\n", out, subject, predicate, object)
        }
        out
    }
}

impl RDFable for Vec<std::path::PathBuf> {
    fn nqd<S: std::fmt::Display>(&self, subject: S, predicate: S) -> String {
        let mut out: String = "".into();
        for object in self {
            out = format!("{}{} {} \"{}\" .\n", out, subject, predicate, object.display())
        }
        out
    }
}

/// Inform query name used in DQL query. Typically used for result extraction
fn extract_query_names(dql: &str) -> Vec<String> {
    let regex = Regex::new(r#"\s*(\w+)\s*\(.*\)\s*\{"#).expect("bogus regexp");
    let mut out: Vec<String> = Vec::new();
    for block in regex.captures_iter(dql) {
        if &block[1] == "var" { continue }
        out.insert(out.len(), block[1].into());
    }
    out
}