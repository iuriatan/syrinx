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
                    l as var(func: eq(Library.name, \"{lib}\"))\n\
                    ar as var(func: {ar_filter})\n\
                    al as var(func: eq(Album.mbid, \"{al_ref}\"))\n\
                    t as var(func: eq(MusicRecording.mbid, \"{t_ref}\"))\n\
                }}\
                mutation {{\
                    set {{\n\
                        uid(l) <Library.track> uid(t) .\n\
                        uid(t) <CreativeWork.title> \"{t_title}\" .\n\
                        uid(t) <CreativeWork.artist> \"{t_ar}\" .\n\
                        uid(t) <CreativeWork.byArtist> uid(ar) .\n\
                        uid(t) <MusicRecording.inAlbum> uid(al) .\n\
                        uid(al) <MusicAlbum.track> uid(t) .\n\
                        {al_title_nqd}\
                        {t_year_nqd}\
                        uid(t) <dgraph.type> \"MusicRecording\" .\n\
                        uid(t) <MusicRecording.mbid> \"{t_ref}\" .\n\
                        {t_dur_nqd}\
                        uid(t) <MusicRecording.sizeKilobytes> \"{t_size}\" .\n\
                        {t_file_nqd}\
                    }}\
                }}\
            }}",
            lib = lib.name,
            ar_filter = track.artists_filter(),
            al_ref = track.album_ref.unwrap_or("".into()),
            al_title_nqd = track.album.nqd("uid(al)", "<CreativeWork.title>"),
            t_ref = track.track_ref,
            t_title = track.title,
            t_ar = track.artist,
            t_year_nqd = track
                .original_year
                .nqd("uid(t)", "<CreativeWork.originalYear>"),
            t_dur_nqd = track
                .duration_seconds
                .nqd("uid(t)", "<MusicRecording.durationSeconds>"),
            t_size = track.file_size,
            t_file_nqd = track.file_path.nqd("uid(t)", "<MusicRecording.file>"),
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

impl Track {
    /// Return filter clauses to select referenced artists
    fn artists_filter(&self) -> String {
        let mut filter: String = "".into();
        for artist in &self.artist_ref {
            if filter.len() > 0 {
                filter = format!("{} OR ", filter);
            }
            filter = format!("{}eq(<Artist.mbid>, \"{}\")", filter, artist);
        }
        filter
    }
}