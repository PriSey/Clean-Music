// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use std::io::{BufReader};
use std::env;
mod player;
mod lyrics;
use player::Player;
mod song;
use song::Song;
use std::sync::{OnceLock, LazyLock};
use tauri::{Emitter, Manager};
use tauri_plugin_fs;
use tauri::WindowEvent;
use serde_json;

static MUSIC_PLAYER: LazyLock<Player> = LazyLock::new(|| Player::new());
static SESSION_THUMBNAIL_CACHE: OnceLock<PathBuf> = OnceLock::new();
static DEFAULT_SONG: OnceLock<PathBuf> = OnceLock::new();
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
    let songs_song = Song::generate_songs(songs, SESSION_THUMBNAIL_CACHE.clone());
    let songs_sorted = MUSIC_PLAYER.sort_songs(songs_song);
    let songs_sorted_songs = songs_sorted;
    for song in songs_sorted_songs{
        emit_command(window.clone(), "CreateSongButtons".to_string(), song);
    }
}

#[tauri::command]
async fn generate_song_front(song:String) -> Song{
    let mut song_vec:Vec<PathBuf> = Vec::new();
    let song_path = PathBuf::from(song);
    song_vec.push(song_path);
    return Song::generate_songs(song_vec, SESSION_THUMBNAIL_CACHE.clone()).first().unwrap().clone();
}

#[tauri::command]
async fn get_lyrics_current() -> String{
  match MUSIC_PLAYER.current(){
    Some(current_song) => return lyrics::get_lyrics(current_song).await,
    _ => return "Lyrics not found".to_string()
  }
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

    MUSIC_PLAYER.queue_dir(Song::generate_songs(dir_paths, SESSION_THUMBNAIL_CACHE.clone()));
}

#[tauri::command]
async fn play_dir(path: String) {
    let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    MUSIC_PLAYER.play_dir(Song::generate_songs(dir_paths, SESSION_THUMBNAIL_CACHE.clone()),false);
}

#[tauri::command]
fn queue_song(path: String) {
    let json_path = SESSION_THUMBNAIL_CACHE.get().unwrap().join("song_cache").with_extension("json");
    let json_file = fs::File::open(&json_path).unwrap();
    let buff_json_file = BufReader::new(&json_file);
    MUSIC_PLAYER.queue_song(Song::generate(PathBuf::from(path), &mut serde_json::from_reader(buff_json_file).unwrap(), SESSION_THUMBNAIL_CACHE.clone()));
}

#[tauri::command]
async fn play_song(path: String, song: String) {
    let dir_paths = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    MUSIC_PLAYER.play_song(Song::generate_songs(dir_paths, SESSION_THUMBNAIL_CACHE.clone()),song);
}

#[tauri::command]
fn write_default_dir(path: String){
  fs::write(DEFAULT_SONG.get().unwrap(), path).unwrap();
}

#[tauri::command]
fn read_default_dir() -> String{
  return  fs::read_to_string(DEFAULT_SONG.get().unwrap()).unwrap();
}

#[tauri::command]
fn play_single_song(path: String){
    let json_path = SESSION_THUMBNAIL_CACHE.get().unwrap().join("song_cache").with_extension("json");
    let json_file = fs::File::open(&json_path).unwrap();
    let buff_json_file = BufReader::new(&json_file);
    MUSIC_PLAYER.play_single_song(Song::generate(PathBuf::from(path), &mut serde_json::from_reader(buff_json_file).unwrap(), SESSION_THUMBNAIL_CACHE.clone()));
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
    let current_song = MUSIC_PLAYER.current();
    match current_song  {
        Some(current_song) => return current_song.path.to_string_lossy().to_string(),
        _ => return "".to_string()
    }
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
            play_single_song,
            write_default_dir,
            read_default_dir,
            get_lyrics_current 
        ])
        .setup(|app| {
            SESSION_THUMBNAIL_CACHE.set(
                app.path().app_cache_dir().unwrap().join("playlist_thumbs")
            ).unwrap();
            DEFAULT_SONG.set(
              app.path().app_data_dir().unwrap().join("default_folder").with_extension("txt")
            ).unwrap();

            fs::remove_dir_all(&SESSION_THUMBNAIL_CACHE.get().unwrap()).ok();
            fs::create_dir_all(&SESSION_THUMBNAIL_CACHE.get().unwrap()).expect("CRASH: failed to create cache dir");

            save_default_thumbnail().unwrap();

            match fs::File::create_new(DEFAULT_SONG.get().unwrap()){
              _ => {}
            }

            
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
