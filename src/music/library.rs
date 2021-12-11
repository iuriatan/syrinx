use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::track::Track;
use crate::dgraph::NodeID;
use crate::CanariaError;
use crate::DgraphClient;

const IGNORED_EXT: [&str; 3] = ["jpg", "jpeg", "png"];

#[derive(Clone, Deserialize)]
pub struct Library {
    /// Node ID from database backend
    pub uid: NodeID,

    /// Library name (unique in Serinus)
    #[serde(alias = "Library.name")]
    pub name: String,

    /// Library canonical (absolute) name in Serinus filesystem
    #[serde(alias = "Library.path")]
    pub path: std::path::PathBuf,

    /// Total duration of libraries audio content
    #[serde(alias = "Library.durationSeconds")]
    pub duration_seconds: u32,

    /// Library size dimensions
    #[serde(alias = "Library.sizeKilobytes")]
    pub size_kilobytes: u64,
}

impl Library {
    /// Import library -> tracks in database fs returning Library struct with
    /// size and duration accountability
    pub async fn new(root: String, name: String, db: &DgraphClient) -> Result<Self, CanariaError> {
        let path = PathBuf::from(root.clone()).canonicalize();
        if let Err(err) = path {
            log::error!("{}: {}", root, err);
            std::process::exit(1);
        }
        let path = path.unwrap();
        let metadata = path.metadata()?;
        if !metadata.is_dir() {
            return Err("library path is not a directory".into());
        }

        let mut lib = db.get_library(name, path.clone()).await?;

        for entry in WalkDir::new(path) {
            let entry = entry.unwrap();
            let entry_path = entry.path();

            let ext = entry_path
                .extension()
                .map(|x| x.to_str().unwrap_or(""))
                .unwrap_or("")
                .to_lowercase();
            if IGNORED_EXT.contains(&ext.as_str()) {
                continue;
            }
            let metadata = entry_path.metadata()?;
            if metadata.is_file() {
                log::info!("importing {}", entry_path.display());
                match Track::from_file(entry_path) {
                    Ok(track) => { db.update_track(track, & mut lib).await?; },
                    Err(err) => log::info!("ignoring {}: {}", entry_path.display(), err),
                }
            }
        }

        Ok(lib)
    }

    /// Return string with rdf triples for database registration
    pub fn rdf(&self) -> String {
        return format!(
            "\
                _:lib <Library.name> \"{}\" .\n\
                _:lib <Library.path> \"{}\" .\n\
                _:lib <Library.durationSeconds> \"{}\" .\n\
                _:lib <Library.sizeKilobytes> \"{}\" .\n\
            ",
            self.name,
            self.path.display(),
            self.duration_seconds,
            self.size_kilobytes,
        );
    }
}