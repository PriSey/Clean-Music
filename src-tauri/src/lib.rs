// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::io::{Write,BufReader};
use std::env;
mod player;
use player::Player;
use player::Song;
use std::sync::{OnceLock, LazyLock};
use tauri::{Emitter, Manager};
use tauri_plugin_fs;
use tauri::WindowEvent;
use lofty::{prelude::*, probe::Probe, config::ParseOptions};
use std::collections::HashMap;
use serde_json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

static MUSIC_PLAYER: LazyLock<Player> = LazyLock::new(|| Player::new());
static SESSION_THUMBNAIL_CACHE: OnceLock<PathBuf> = OnceLock::new();
const DEFAULT_SVG_BYTES: &[u8] = include_bytes!("../music-file-default.svg");

#[derive(Clone, serde::Serialize, Debug)]
struct ButtonPayload {
    id: String,
    text: String,
    class: String,
}



fn pull_paths(path: PathBuf) -> Vec<ButtonPayload> {
    let mut buttons = Vec::new();
    let paths = fs::read_dir(path).unwrap();
    let accepted_paths = "mp3wavflacogg".to_string();
    for path in paths {
        let proper_path = path.unwrap().path();
        let path_extension = proper_path.extension().unwrap_or(OsStr::new("txt"));
        if !proper_path.is_dir() && !accepted_paths.contains(path_extension.to_str().unwrap()) {
            continue;
        } else {
            let payload = ButtonPayload {
                id: proper_path.to_string_lossy().to_string(),
                text: proper_path
                    .to_string_lossy()
                    .to_string()
                    .split("/")
                    .last()
                    .unwrap()
                    .to_string(),
                class: "LeftDirectoryButton".to_string(),
            };
            buttons.push(payload);
        }
    }

    return buttons;
}

fn emit_command<T: ::serde::Serialize + std::clone::Clone>(
    window: tauri::Window,
    title: String,
    payload: T,
) {
    let _ = window.emit(&title, payload);
}

#[tauri::command]
async fn fetch_songs(window: tauri::Window, path: String) {
    let songs = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    let songs_song = generate_songs(songs);
    let songs_sorted = MUSIC_PLAYER.sort_songs(songs_song);
    let songs_sorted_songs = generate_songs(songs_sorted);
    for song in songs_sorted_songs{
        emit_command(window.clone(), "CreateSongButtons".to_string(), song);
    }
}

#[tauri::command]
async fn generate_song_front(song:String) -> Song{
    let mut song_vec:Vec<PathBuf> = Vec::new();
    let song_path = PathBuf::from(song);
    song_vec.push(song_path);
    return generate_songs(song_vec).first().unwrap().clone();
}

fn generate_song(song: PathBuf, songs_known: &mut HashMap<PathBuf, Song>) -> Song{
    

    if let Some(song_found) = songs_known.get(&song) {
        return song_found.clone();
    }
    let start = std::time::Instant::now();
    let tagged_file = Probe::open(song.clone())
    .unwrap()
    .options(ParseOptions::new().read_properties(false))
    .read()
    .unwrap();
        let tags_wrapped = match tagged_file.primary_tag() {
            Some(primary_tag) => Some(primary_tag),
            None => tagged_file.first_tag(),
        };
        let mut hasher = DefaultHasher::new();
        song.file_name().unwrap().hash(&mut hasher);
        let title;
        let track_number;
        let album;
        let thumbnail;
        match tags_wrapped{
            Some(tag) => {
                title = tag.title().unwrap_or(song.file_name().unwrap().to_string_lossy());
                album = tag.album().unwrap_or(song.file_name().unwrap().to_string_lossy()).to_string();
                track_number = tag.track().unwrap_or(hasher.finish() as u32);
                let thumbnail_image = tag.pictures().first();
                match thumbnail_image {
                    Some(image) => {
                         let extension = match image.mime_type().expect("Failure in fetching extension").to_string().as_str() {
                            "image/png" => "png",
                            "image/jpeg" | "image/jpg" => "jpg",
                            _ => "bin", // Fallback for unknown formats
                        };
                        let image_filename = SESSION_THUMBNAIL_CACHE.get().unwrap()
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
                        thumbnail = SESSION_THUMBNAIL_CACHE.get().unwrap().join("music-file-default").with_extension("svg");
                    }
                }
            }
            None => {
                title = song.file_name().unwrap().to_string_lossy();
                album = song.file_name().unwrap().to_string_lossy().to_string();
                track_number = hasher.finish() as u32;
                thumbnail = SESSION_THUMBNAIL_CACHE.get().unwrap().join("music-file-default").with_extension("svg");
            }
        }
        let song_proccessed = Song{
            path: song.to_string_lossy().to_string(),
            title:title.to_string(),
            thumbnail:thumbnail.to_string_lossy().to_string(),
            album: album,
            track_number: track_number
        };
        
        songs_known.insert(song,song_proccessed.clone());
        
        return song_proccessed;
}

fn generate_songs(songs: Vec<PathBuf>) -> Vec<Song>{
    let json_path = SESSION_THUMBNAIL_CACHE.get().unwrap().join("song_cache").with_extension("json");
    let json_file = fs::File::open(&json_path).unwrap();
    let buff_json_file = BufReader::new(&json_file);
    let mut songs_known: HashMap<PathBuf, Song> = serde_json::from_reader(buff_json_file).unwrap();

    let songs_processed: Vec<Song> = songs.into_iter().map(|p| generate_song(p,&mut songs_known)).collect();
    let new_json = serde_json::to_string(&songs_known).unwrap();
    fs::write(json_path, new_json).unwrap();

    return songs_processed;
}

#[tauri::command]
fn load_from_music_folder(window: tauri::Window, path: String) {
    let buttons = pull_paths(PathBuf::from(path));

    for button in buttons {
        emit_command(window.clone(), "LeftbarDirButtons".to_string(), button);
    }
}
#[tauri::command]
fn check_dir(path: String) -> bool {
    let pb = PathBuf::from(path);
    return pb.is_dir();
}

#[tauri::command]
fn queue_dir(path: String) {
    let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));

    MUSIC_PLAYER.queue_dir(generate_songs(dir_paths));
}

#[tauri::command]
async fn play_dir(path: String) {
    let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    MUSIC_PLAYER.play_dir(generate_songs(dir_paths),false);
}

#[tauri::command]
fn queue_song(path: String) {
    MUSIC_PLAYER.queue_song(PathBuf::from(path));
}

#[tauri::command]
async fn play_song(path: String, song: String) {
    let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    MUSIC_PLAYER.play_song(generate_songs(dir_paths),song);
}


#[tauri::command]
fn play_single_song(path: String){
    MUSIC_PLAYER.play_single_song(PathBuf::from(path));
}

// #[tauri::command]
// fn play__song_raw(ong: String) {
//     let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));
//     MUSIC_PLAYER.play_song(generate_songs(dir_paths),song);
// }

#[tauri::command]
fn send_command(cmd: usize) {
    MUSIC_PLAYER.send_command(cmd);
}

#[tauri::command]
fn set_volume(volume: String) {
    MUSIC_PLAYER.set_volume(volume.parse::<usize>().unwrap());
}

#[tauri::command]
async fn get_position() -> f32 {
    let total_pos = Duration::as_millis(&MUSIC_PLAYER.get_song_duration()) as f32;
    let real_pos = MUSIC_PLAYER.get_position() as f32;
    let virt_pos = (real_pos / total_pos) * 1000.0;
    return virt_pos;
}

#[tauri::command]
async fn get_up() -> bool {
    return MUSIC_PLAYER.get_playing();
}

#[tauri::command]
fn seek_position(virtual_position: f32) {
    let total_pos = Duration::as_millis(&MUSIC_PLAYER.get_song_duration()) as f32;
    let real_pos = (virtual_position / 1000.0) * total_pos;
    MUSIC_PLAYER.seek(real_pos as usize);
}
#[tauri::command]
async fn get_current_song() -> String {
    let current_path_dir = MUSIC_PLAYER.current();
    let song = current_path_dir.to_string_lossy().to_string();
    return song;
}

fn save_default_thumbnail() -> std::io::Result<()> {
    let dest_path = SESSION_THUMBNAIL_CACHE.get().unwrap().join("music-file-default").with_extension("svg");
    let json_path = SESSION_THUMBNAIL_CACHE.get().unwrap().join("song_cache").with_extension("json");

    if !dest_path.exists(){
        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)?;        
        }

        std::fs::write(dest_path, DEFAULT_SVG_BYTES)?;
    }

    if !json_path.exists(){
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent)?;        
        }

        std::fs::write(&json_path, "{}")?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::sync::LazyLock::force(&MUSIC_PLAYER);

    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            load_from_music_folder,
            fetch_songs,
            queue_dir,
            queue_song,
            send_command,
            play_song,
            play_dir,
            set_volume,
            check_dir,
            get_position,
            get_up,
            seek_position,
            get_current_song,
            generate_song_front,
            play_single_song
            
        ])
        .setup(|app| {
            SESSION_THUMBNAIL_CACHE.set(
                app.path().app_cache_dir().unwrap().join("playlist_thumbs")
            ).unwrap();

            fs::remove_dir_all(&SESSION_THUMBNAIL_CACHE.get().unwrap()).ok();
            fs::create_dir_all(&SESSION_THUMBNAIL_CACHE.get().unwrap()).expect("CRASH: failed to create cache dir");

            save_default_thumbnail().unwrap();


            
            Ok(())
        })
        .on_window_event(|window, event|{
            if let WindowEvent::Destroyed = event {
                let app = window.app_handle();
                fs::remove_dir_all(&*SESSION_THUMBNAIL_CACHE.get().unwrap()).ok();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
        
}
