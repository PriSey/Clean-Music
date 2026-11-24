use std::path::PathBuf;
use std::collections::HashMap;
use std::fs;
use std::time::Duration;
use std::io::{Write, BufReader};
use std::sync::OnceLock;
use std::borrow::Cow;
use lofty::{prelude::*, probe::Probe, config::ParseOptions};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct Song {
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub thumbnail: String,
    pub album: String,
    pub track_number: u32,
    pub duration: Duration
}

impl Song{
  pub fn generate(song: PathBuf, songs_known: &mut HashMap<PathBuf, Song>, session_thumbnail_cache: OnceLock<PathBuf>) -> Song{
    if let Some(song_found) = songs_known.get(&song) {
        return song_found.clone();
    }
    let tagged_file = Probe::open(song.clone())
    .unwrap()
    .options(ParseOptions::new().read_properties(true))
    .read()
    .unwrap();
    let tags_wrapped = match tagged_file.primary_tag() {
        Some(primary_tag) => Some(primary_tag),
        None => tagged_file.first_tag(),
    };
    let mut hasher = DefaultHasher::new();
    let duration = tagged_file.properties().duration();
    let binding = song.clone();
    let filename = binding.file_name().unwrap().to_string_lossy();
    &song.file_name().unwrap().hash(&mut hasher);
    let title;
    let track_number;
    let album;
    let artist;
    let thumbnail;
    match tags_wrapped{
        Some(tag) => {
            title = tag.title().unwrap_or(Cow::Borrowed(&filename));
            album = tag.album().unwrap_or(Cow::Borrowed(&filename)).to_string();
            artist = tag.artist().unwrap_or(Cow::Borrowed(&song.file_name().unwrap().to_string_lossy())).to_string();
            track_number = tag.track().unwrap_or(hasher.finish() as u32);
            let thumbnail_image = tag.pictures().first();
            match thumbnail_image {
                Some(image) => {
                    let extension = match image.mime_type().expect("Failure in fetching extension").to_string().as_str() {
                        "image/png" => "png",
                        "image/jpeg" | "image/jpg" => "jpg",
                        _ => "bin", // Fallback for unknown formats
                    };
                    let image_filename = session_thumbnail_cache.get().unwrap()
                        .join(title.to_string()
                        .replace('/', "")
                        .replace("\\",""))
                        .with_extension(extension);
                    if !image_filename.exists(){
                        let mut file = fs::File::create(&image_filename).unwrap();
                        file.write_all(image.data()).expect("Failed to write to thumbnail");
                    } else {
                    }
                    
                    thumbnail = image_filename;
                }
                None => {
                    thumbnail = session_thumbnail_cache.get().unwrap().join("music-file-default").with_extension("svg");
                }
            }
        }
        None => {
            title = Cow::Borrowed(&filename);
            album = song.file_name().unwrap().to_string_lossy().to_string();
            artist = "unknown".to_string();
            track_number = hasher.finish() as u32;
            thumbnail = session_thumbnail_cache.get().unwrap().join("music-file-default").with_extension("svg");
        }
    }
    let song_proccessed = Song{
        path: song.clone(),
        title: title.to_string(),
        artist: artist,
        thumbnail:thumbnail.to_string_lossy().to_string(),
        album: album,
        track_number: track_number,
        duration: duration
    };
    
    songs_known.insert(song,song_proccessed.clone());
    
    return song_proccessed;
  }

  pub fn generate_songs(songs: Vec<PathBuf>, session_thumbnail_cache: OnceLock<PathBuf>) -> Vec<Song>{
    let json_path = session_thumbnail_cache.get().unwrap().join("song_cache").with_extension("json");
    let json_file = fs::File::open(&json_path).unwrap();
    let buff_json_file = BufReader::new(&json_file);
    let mut songs_known: HashMap<PathBuf, Song> = serde_json::from_reader(buff_json_file).unwrap();
    let songs_processed: Vec<Song> = songs.into_iter().map(|p| Song::generate(p,&mut songs_known,session_thumbnail_cache.clone())).collect();
    let new_json = serde_json::to_string(&songs_known).unwrap();
    fs::write(json_path, new_json).unwrap();

    return songs_processed;
  }
}