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
            
        match result {
            Some(lib) => Ok(lib),
            None => Err("library update/insertion failed".into())
        }
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
                    {ar_vars}\
                    al as var(func: eq(Album.mbid, \"{al_ref}\"))\n\
                    au as var(func: eq(AudioObject.filepath, \"{au_ref}\"))
                    t as var(func: eq(MusicRecording.mbid, \"{t_ref}\"))\n\
                }}\n\
                mutation {{\
                    set {{\n\
                        uid(l) <Library.track> uid(t) .\n\
                        {ar_muts_nqd}\
                        uid(al) <dgraph.type> \"MusicAlbum\" .\n\
                        uid(al) <MusicAlbum.track> uid(t) .\n\
                        {al_title_nqd}\
                        uid(t) <dgraph.type> \"MusicRecording\" .\n\
                        uid(t) <MusicRecording.mbid> \"{t_ref}\" .\n\
                        uid(t) <CreativeWork.title> \"{t_title}\" .\n\
                        uid(t) <CreativeWork.artist> \"{t_ar}\" .\n\
                        uid(t) <MusicRecording.inAlbum> uid(al) .\n\
                        uid(t) <MusicRecording.audio> uid(au) .\n\
                        {t_dur_nqd}\
                        {t_year_nqd}\
                        uid(au) <dgraph.type> \"AudioObject\" . \n\
                        uid(au) <AudioObject.sizeKilobytes> \"{t_size}\" .\n\
                        uid(au) <AudioObject.filepath> \"{au_ref}\" .\n\
                    }}\
                }}\
            }}",
            lib = lib.name,
            ar_vars = track.artists_vars(),
            ar_muts_nqd = track.artists_muts("uid(l)", "uid(t)"),
            al_ref = track.album_ref.unwrap_or("".into()),
            al_title_nqd = track.album.nqd("uid(al)", "<CreativeWork.title>"),
            au_ref = track.file_path.to_string_lossy(),
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
        );
        self.mutate(dql.as_str()).await?;

        let result = self
            .query_single(format!(
                "{{\
                    q(func: eq(<MusicRecording.mbid>, \"{}\")) @normalize {{\n\
                        title: CreativeWork.title\n\
                        artist: CreativeWork.artist\n\
                        artist_ref: Artist.mbid\n\
                        original_year: CreativeWork.originalYear\n\
                        track_ref: MusicRecording.mbid\n\
                        <MusicRecording.audio> {{\n\
                            file_path: AudioObject.filepath\n\
                            file_size: AudioObject.sizeKilobytes\n\
                        }}\
                    }}\
                }}",
                track.track_ref,
            ).as_str())
            .await?;

        match result {
            Some(track) => Ok(track),
            None => Err("track insertion/update failed".into())
        }
    }
}

impl Track {
    /// Return filter clauses to select referenced artists
    fn artists_vars(&self) -> String {
        let mut filter: String = "".into();
        for (index, artist) in self.artist_ref.iter().enumerate() {
            filter = format!("ar{} as var(func: eq(<Artist.mbid>, \"{}\"))\n", index, artist);
        }
        filter
    }
    
    fn artists_muts(&self, lib_subject: &str, track_subject: &str) -> String {
        let mut artists_names: Vec<String> = Vec::new();
        if self.artist_ref.len() > 1 {
            // TODO: find each artist name
            log::debug!("track artist (string): {}", self.artist);
            log::debug!("track reference (vec): {:?}", self.artist_ref);
            log::error!("multi artists track not supported. repeating artist string");
            for _ in self.artist_ref.iter() {
                artists_names.insert(artists_names.len(), self.artist.clone());
            }
        } else {
            artists_names.insert(0, self.artist.clone())
        }

        // grant artist_ref.len() == artists_names.len()
        let mut out = String::from("");
        for (index, artist) in self.artist_ref.iter().enumerate() {
            out = format!("{}\
                    {lib} <Library.artist> uid(ar{index}) .\n\
                    uid(ar{index}) <dgraph.type> \"Artist\" .\n\
                    uid(ar{index}) <Artist.mbid> \"{reference}\" .\n\
                    uid(ar{index}) <Artist.names> \"{name}\" .\n\
                    {track} <CreativeWork.byArtist> uid(ar{index}) .\n\
                ",
                out,
                lib = lib_subject,
                track = track_subject,
                index = index,
                reference = artist,
                name = artists_names[index]
            )
        }
        out
    }
}