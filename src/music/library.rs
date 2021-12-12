use serde::Deserialize;
use std::path::PathBuf;
use walkdir::WalkDir;

use super::track::Track;
use crate::CanariaError;
use crate::DgraphClient;


#[derive(Clone, Deserialize)]
pub struct Library {
    /// Library name (unique in Serinus)
    pub name: String,

    /// Library canonical (absolute) name in Serinus filesystem
    pub path: std::path::PathBuf,
    
    #[serde(default)]
    pub duration_seconds: u32,
    
    #[serde(default)]
    pub size_kilobytes: u64,
}

impl Library {
    /// Import library -> tracks in database fs returning Library struct with
    /// size and duration accountability
    pub async fn new(
        root: String,
        name: String,
        db: &DgraphClient,
        music_ignore_list: Vec<String>,
    ) -> Result<Self, CanariaError> {
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
            if music_ignore_list.contains(&ext) {
                continue;
            }
            let metadata = entry_path.metadata()?;
            if metadata.is_file() {
                log::info!("importing {}", entry_path.display());
                match Track::from_file(entry_path) {
                    Ok(track) => { 
                        db.update_track(track, & mut lib).await?;
                    },
                    Err(err) => log::info!("ignoring {}: {}", entry_path.display(), err),
                }
            }
        }

        Ok(lib)
    }
}
