// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use std::path::PathBuf;
use std::ffi::OsStr;
mod player;
use player::Player;
use tauri::Emitter;
use std::sync::LazyLock;

static MUSIC_PLAYER: LazyLock<Player> = LazyLock::new(||Player::new());

#[derive(Clone, serde::Serialize)]
#[derive(Debug)]
struct ButtonPayload {
    id: String,
    text: String,
    class: String,
}

fn pull_paths(path: PathBuf) -> Vec<ButtonPayload>{
    let mut buttons = Vec::new();
    let paths = fs::read_dir(path).unwrap();
    let accepted_paths = "mp3wavflacogg".to_string();
    for path in paths {
        let proper_path = path.unwrap().path();
        let path_extension = proper_path.extension().unwrap_or(OsStr::new("txt"));
        if !proper_path.is_dir() && !accepted_paths.contains(path_extension.to_str().unwrap()){
            continue
        }
        else{
            let payload = ButtonPayload {
                id: proper_path.to_string_lossy().to_string(),
                text:proper_path.to_string_lossy().to_string().split("/").last().unwrap().to_string(),
                class: "LeftDirectoryButton".to_string()
            };
            buttons.push(payload);
        }
    }

    return buttons;
}

fn emit_command<T: ::serde::Serialize + std::clone::Clone>(window: tauri::Window, title:String, payload:T){
    let _= window.emit(&title, payload);
}

#[tauri::command]
fn fetch_songs(window: tauri::Window, path:String){
    let songs = MUSIC_PLAYER.load_dir(PathBuf::from(path));
    for song in songs{
        let song_button = ButtonPayload{
            id: song.to_string_lossy().to_string(),
            text: song.to_string_lossy().split("/").last().unwrap().to_string(),
            class: "RightbarSongButton".to_string()
        };
        emit_command(window.clone(), "CreateSongButtons".to_string(), song_button);
    }
}

#[tauri::command]
fn load_from_music_folder(window: tauri::Window, path:String){
    let buttons = pull_paths(PathBuf::from(path));

    for button in buttons{
        emit_command(window.clone(), "LeftbarDirButtons".to_string(), button);
    }
}
#[tauri::command]
fn check_dir(path:String) -> bool{
    let pb = PathBuf::from(path);
    return pb.is_dir();
}

#[tauri::command]
fn queue_dir(path:String){
    MUSIC_PLAYER.queue_dir(PathBuf::from(path));
}

#[tauri::command]
fn play_dir(path:String){
    MUSIC_PLAYER.play_dir(PathBuf::from(path));
}

#[tauri::command]
fn queue_song(path:String){
    MUSIC_PLAYER.queue_song(PathBuf::from(path));
}

#[tauri::command]
fn play_song(path:String){
    MUSIC_PLAYER.play_song(PathBuf::from(path));
}

#[tauri::command]
fn send_command(cmd:String){
    let command = cmd.chars().last().unwrap();
    MUSIC_PLAYER.send_command(command.to_digit(10).unwrap() as usize);
}

#[tauri::command]
fn set_volume(volume:String){
    MUSIC_PLAYER.set_volume(volume.parse::<usize>().unwrap());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {

    std::sync::LazyLock::force(&MUSIC_PLAYER); 

    tauri::Builder::default()
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
            ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
