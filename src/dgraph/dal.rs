use std::path::PathBuf;

use super::DgraphClient;
use crate::music::library::Library;
use crate::music::track::Track;
use crate::CanariaError;

impl DgraphClient {
    // MARK: Business related methods

    /// Provides Library struct up2date with persistent data
    pub async fn get_library(&self, name: String, path: PathBuf) -> Result<Library, CanariaError> {
        let lib_query = format!(
            "q(func: eq(<Library.name>, \"{}\")){{\
                uid,\
                Library.name,\
                Library.path,\
                Library.durationSeconds,\
                Library.sizeKilobytes\
            }}",
            name
        );

        match self.query_single::<Library>(lib_query.as_str()).await? {
            Some(lib) => Ok(lib),
            None => {
                let lib = Library {
                    uid: "0".into(),
                    name: name.clone(),
                    path: path.clone(),
                    duration_seconds: 0,
                    size_kilobytes: 0,
                };
                self.mutate(lib.rdf().as_str()).await?;
                Ok(self
                    .query_single::<Library>(lib_query.as_str())
                    .await
                    .unwrap()
                    .unwrap())
            }
        }
    }
    /// Provides Track struct up2date with persistent data
    pub async fn update_track(
        &self,
        track: Track,
        lib: &mut Library,
    ) -> Result<Track, CanariaError> {
        let track_query = format!(
            "q(func: eq(MusicRecording.file, \"{}\")){{\
                CreativeWork.title,\
                CreativeWork.artist,\
                MusicRecording.file,\
            }}",
            track.file_path.display()
        );
        match self.query_single::<Track>(track_query.as_str()).await? {
            Some(db_track) => {
                // TODO: merge differences track x db_track
                Ok(db_track)
            }
            None => {
                lib.duration_seconds += track.duration_seconds.unwrap_or(0);
                lib.size_kilobytes += track.file_size;
                let trimmed_path = track
                    .file_path
                    .strip_prefix(lib.path.to_str().unwrap_or(""))?;
                let rdf = format!(
                    "\
                    <{}> <Library.track> _:track .\n\
                    <{0}> <Library.durationSeconds> \"{}\" .\n\
                    <{0}> <Library.sizeKilobytes> \"{}\".\n\
                    _:track <CreativeWork.title> \"{}\" .\n\
                    _:track <CreativeWork.artist> \"{}\" .\n\
                    _:track <MusicRecording.file> \"{}\" .\n",
                    lib.uid,
                    lib.duration_seconds,
                    lib.size_kilobytes,
                    track.title,
                    track.artist,
                    trimmed_path.display(),
                );
                self.mutate(rdf.as_str()).await?;
                Ok(track)
            }
        }
    }
}
