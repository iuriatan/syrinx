use serde::Deserialize;

#[derive(Deserialize)]
pub struct Artist {
    /// MusicBrainz ID
    pub mbid: Option<String>,

    /// List of names the artist is known as
    pub names: Vec<String>,
}
impl Artist {
    pub fn new(name: String, mbid: Option<String>) -> Self {
        Self {
            mbid,
            names: vec![name],
        }
    } 
}