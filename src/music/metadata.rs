use log::{debug, warn};
use phf::phf_map;
use std::path::Path;
use symphonia::core::io::MediaSourceStream;

use super::track::Track;
use crate::CanariaError;

const DEBUG_MUSIC_METADATA: bool = true;
const UNINITIALIZED_STR: &str = "_nihil_";

impl Track {
    fn new(filepath: &Path) -> Self {
        let file_path = vec![filepath.canonicalize().unwrap().into()];
        let file_size = filepath.metadata().unwrap().len() / 1024;
        Self {
            title: UNINITIALIZED_STR.into(),
            artist: UNINITIALIZED_STR.into(),
            artist_ref: Vec::new(),
            original_year: None,
            album: None,
            album_ref: None,
            tags: Vec::new(),
            track_ref: UNINITIALIZED_STR.into(),
            duration_seconds: None,
            file_path,
            file_size,
        }
    }
    fn set_field(&mut self, field: &str, value: String) {
        match field {
            "title" => self.title = value,
            "artist" => self.artist = value,
            "artist_id" | "artist_ref" => self.artist_ref.push(value),
            "original_year" => self.original_year = value.parse().ok(),
            "album" => self.album = Some(value),
            "album_id" | "album_ref" => self.album_ref = Some(value),
            "tags" => self.tags.push(value),
            "track_id" => self.track_ref = value,
            _ => warn!("trying to set unexpected metadata field `{}`", field),
        }
    }
}

static TAG_X_MAP: phf::Map<&'static str, &'static str> = phf_map! {
    // From ID3v2 cases
    "TIT2" => "title",
    "TPE1" => "artist",
    "TALB" => "album",
    "TORY" => "original_year",
    "TXXX:MusicBrainz Album Id" => "album_id",
    "TXXX:MusicBrainz Artist Id" => "artist_id",
    "TXXX:MusicBrainz Release Track Id" => "track_id",
    // From VorbisComment cases
    "TITLE" => "title",
    "ARTIST" => "artist",
    "ALBUM" => "album",
    "ORIGINALYEAR" => "original_year",
    "MUSICBRAINZ_ALBUMID" => "album_id",
    "MUSICBRAINZ_ARTISTID" => "artist_id",
    "MUSICBRAINZ_RELEASETRACKID" => "track_id",
};

pub fn extract_metadata(file: &Path) -> Result<Track, CanariaError> {
    let file_ext = file.extension();
    if file_ext.is_none() {
        return Err("audio files must have name extension".into());
    }
    let file_ext = file_ext.unwrap().to_str().unwrap();

    let f = std::fs::File::open(file)?;
    let mss = MediaSourceStream::new(Box::new(f), Default::default());
    let mut hint = symphonia::core::probe::Hint::new();
    hint.with_extension(file_ext);

    let result = match symphonia::default::get_probe().format(
        &hint,
        mss,
        &Default::default(),
        &Default::default(),
    ) {
        Ok(mut probed) => {
            // TODO: Unify first 2 as soon as if let chains get implemented
            if let Some(metadata_rev) = probed.format.metadata().current() {
                // TODO: define audio duration
                extract_tags(metadata_rev, file)
            } else if let Some(metadata_rev) =
                probed.metadata.get().as_ref().and_then(|m| m.current())
            {
                extract_tags(metadata_rev, file)
            } else {
                return Err("symphonia probed no metadata".into());
            }
        }
        Err(err) => return Err(format!("metadata extraction fail: {}", err).into()),
    }?;

    quality_control(result)
}

use symphonia::core::meta::MetadataRevision;

fn extract_tags(md_rev: &MetadataRevision, file: &Path) -> Result<Track, CanariaError> {
    let tags = md_rev.tags();
    let mut out = Track::new(file);

    for tag in tags.iter() {
        if DEBUG_MUSIC_METADATA {
            debug!("probed tag {}: {}", &tag.key, &tag.value);
        }
        if let Some(md_field) = TAG_X_MAP.get(tag.key.as_str()) {
            out.set_field(md_field, tag.value.to_string())
        }
    }
    Ok(out)
}

/// Ensures track meet metadata quality standards
fn quality_control(track: Track) -> Result<Track, CanariaError> {
    // fail 
    if track.artist == UNINITIALIZED_STR
        || track.artist == ""
        || track.title == UNINITIALIZED_STR
        || track.title == ""
    {
        return Err("poor metadata: artist and/or title".into());
    }
    
    if track.track_ref == UNINITIALIZED_STR || track.track_ref == "" {
        log::warn!("uncatalogued track")
    }
    
    if track.artist_ref.is_empty() {
        log::warn!("uncatalogued artist")
    }

    Ok(track)
}
