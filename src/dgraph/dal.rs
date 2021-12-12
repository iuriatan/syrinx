use std::path::PathBuf;

use super::DgraphClient;
use super::RDFable;
use crate::music::library::Library;
use crate::music::track::Track;
use crate::CanariaError;

impl DgraphClient {
    // MARK: Business related methods

    /// Provides Library struct up2date with persistent data
    pub async fn get_library(&self, name: String, path: PathBuf) -> Result<Library, CanariaError> {
        let upsert_lib = format!(
            "upsert {{\
                query {{\
                    q(func: eq(<Library.name>, \"{}\")){{\
                        lib as uid\
                    }}\
                }}\
                mutation {{\
                    set {{\n\
                        uid(lib) <dgraph.type> \"Library\".\n\
                        uid(lib) <Library.name> \"{0}\" .\n\
                        uid(lib) <Library.path> \"{}\" .\n\
                    }}\
                }}\
            }}",
            name,
            path.display(),
        );

        self.mutate(upsert_lib.as_str()).await?;

        let result = self.query_single::<Library>(
            format!(
                "{{\
                    var(func: eq(<Library.name>, \"{}\")) {{\n\
                        <Library.track> {{\n\
                            d as MusicRecording.durationSeconds\n\
                            s as MusicRecording.sizeKilobytes\n\
                        }}\n\
                        sumS as sum(val(s))\n\
                        sumD as sum(val(d))\n\
                    }}\n\
                    q(func: eq(<Library.name>, \"{0}\")) {{\n\
                        name : Library.name\n\
                        path: Library.path\n\
                        duration_seconds: val(sumD)\n\
                        size_kilobytes: val(sumS)\n\
                    }}\
                }}",
                name
            ).as_str()).await?;
        Ok(result.unwrap())
    }

    /// Provides Track struct up2date with persistent data
    pub async fn update_track(
        &self,
        track: Track,
        lib: &mut Library,
    ) -> Result<Track, CanariaError> {
        let dql = format!(
            "\
            upsert {{\
                query {{\
                    t as var(func: eq(MusicRecording.mbid, \"{}\"))\
                    l as var(func: eq(Library.name, \"{}\"))\
                }}\
                mutation {{\
                    set {{\n\
                        uid(l) <Library.track> uid(t) .\n\
                        uid(t) <CreativeWork.title> \"{}\" .\n\
                        uid(t) <CreativeWork.artist> \"{}\" .\n\
                        {}\
                        uid(t) <dgraph.type> \"MusicRecording\" .\n\
                        uid(t) <MusicRecording.mbid> \"{0}\" .\n\
                        {}\
                        uid(t) <MusicRecording.sizeKilobytes> \"{}\" .\n\
                        {}\
                    }}\
                }}\
            }}",
            track.track_ref,
            lib.name,
            track.title,
            track.artist,
            track
                .original_year
                .nqd("uid(t)", "<CreativeWork.originalYear>"),
            track
                .duration_seconds
                .nqd("uid(t)", "<MusicRecording.durationSeconds>"),
            track.file_size,
            track.file_path.nqd("uid(t)", "<MusicRecording.file>"),
        );
        self.mutate(dql.as_str()).await?;

        let result = self
            .query_single(format!(
                "{{\
                    q(func: eq(<MusicRecording.mbid>, \"{}\")) {{\n\
                        title: CreativeWork.title\n\
                        artist: CreativeWork.artist\n\
                        artist_ref: Artist.mbid\n\
                        original_year: CreativeWork.originalYear\n\
                        track_ref: MusicRecording.mbid\n\
                        file_path: MusicRecording.file\n\
                        file_size: MusicRecording.sizeKilobytes\n\
                    }}\
                }}",
                track.track_ref,
            ).as_str())
            .await?;
        Ok(result.unwrap())
    }
}
