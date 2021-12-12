use std::path::{Path, PathBuf};
use serde::Deserialize;

use crate::CanariaError;
use super::metadata;

#[derive(Clone,Debug,Deserialize)]
pub struct Track {
    /// Music title as should be displayed in music player
    pub title: String,

    /// Music artist as should be displayed in the music player
    pub artist: String,
    
    /// A list of artist's ID`s referenced
    #[serde(skip)]
    pub artist_ref: Vec<String>,
    
    /// Tracks original release year
    pub original_year: Option<u16>,
    
    /// The album (if any) that track belongs to
    #[serde(skip)]
    pub album: Option<String>,
    
    /// The ID of the album mentioned by "album" field
    #[serde(skip)]
    pub album_ref: Option<String>,
    
    /// The list of tags associated with that track
    #[serde(skip)]
    pub tags: Vec<String>,
    
    /// Recording ID, the real specification of a track
    pub track_ref: String,
    
    /// Playback duration in seconds
    pub duration_seconds: Option<u32>,
    
    /// There may be multiple files for a same recording
    pub file_path: Vec<PathBuf>,
    
    /// File size in bytes
    pub file_size: u64,
}

impl Track {
    pub fn from_file(path: &Path) -> Result<Self, CanariaError> {
        let t_from_file = metadata::extract_metadata(path)?;
        log::debug!("extracted {:#?}", t_from_file);
        Ok(t_from_file)
    }
}
