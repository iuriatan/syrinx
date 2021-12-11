use std::path::{Path, PathBuf};
use serde::Deserialize;

use crate::CanariaError;
use super::metadata;

#[derive(Clone,Debug,Deserialize)]
pub struct Track {
    /// Music title as should be displayed in music player
    #[serde(rename = "CreativeWork.title")]
    pub title: String,

    /// Music artist as should be displayed in the music player
    #[serde(rename = "CreativeWork.artist")]
    pub artist: String,
    
    /// A list of artist's ID`s referenced
    #[serde(rename = "CreativeWork.byArtist")]
    pub artist_ref: Vec<String>,
    
    /// Tracks original release year
    #[serde(rename = "CreativeWork.originalYear")]
    pub original_year: Option<u16>,
    
    /// The album (if any) that track belongs to
    #[serde(rename = "MusicRecording.album")]
    pub album: Option<String>,
    
    /// The ID of the album mentioned by "album" field
    #[serde(rename = "MusicAlbum.mbid")]
    pub album_ref: Option<String>,
    
    /// The list of tags associated with that track
    pub tags: Vec<String>,
    
    /// Recording ID, the real specification of a track
    #[serde(rename = "MusicRecording.mbid")]
    pub track_ref: String,
    
    /// Playback duration in seconds
    #[serde(rename = "MusicRecording.durationSeconds")]
    pub duration_seconds: Option<u32>,
    
    /// File path to the audio
    #[serde(rename = "MusicRecording.file")]
    pub file_path: PathBuf,
    
    /// File size in bytes
    #[serde(rename = "MusicRecording.sizeKilobytes")]
    pub file_size: u64,
}

impl Track {
    pub fn from_file(path: &Path) -> Result<Self, CanariaError> {
        let t_from_file = metadata::extract_metadata(path)?;
        log::debug!("extracted {:#?}", t_from_file);
        Ok(t_from_file)
    }
}
