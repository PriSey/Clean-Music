use std::{collections::HashMap, fs, fs::ReadDir, path::PathBuf, time::Duration};
mod queue;
use crate::song::Song;
use queue::Queue;




#[derive(Clone)]
pub struct Player {
    queue: Queue,
}

impl Player {
    pub fn new() -> Self {
        let queue = Queue::new();

        return Player { queue };
    }

    pub fn send_command(&self, command: usize) {
        match command {
            0 => self.queue.pause(),
            1 => self.queue.play(),
            2 => self.queue.skip_forward(),
            3 => self.queue.skip_backward(),
            4 => self.queue.reset(),
            5 => self.queue.loop_song(),
            6 => self.queue.loop_stop(),
            _ => println!("How did you get here?"),
        }
    }

    pub fn play_song(&self, dir: Vec<Song>, song: String) {
        self.queue.clear();
        let dir_vec = self.sort_songs(dir.clone());
        let mut position:usize = 0;
        for i in 0..dir_vec.len(){
          if *dir_vec.get(i).unwrap().path.to_string_lossy() == song{
            position = i;
            break;
          }
        }

        self.queue.go_to(position);

        self.play_dir(dir, true);
    }

    pub fn play_single_song(&self, song: Song) {
        self.queue.clear();
        self.queue.add_songs(self.load_song(song));
    }

    pub fn queue_song(&self, song: Song) {
        self.queue.add_songs(self.load_song(song));
    }

    pub fn play_dir(&self, dir: Vec<Song>, from_i: bool) {
        if from_i{
          self.queue.clear();
        } else {
          self.queue.reset();
        }
        let dir_vec = self.sort_songs(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn queue_dir(&self, dir: Vec<Song>) {
        let dir_vec = self.sort_songs(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn load_song(&self, song: Song) -> Vec<Song> {
        let mut command = Vec::new();
        command.push(song);
        return command;
    }

    pub fn load_dir(&self, directory: PathBuf) -> Vec<PathBuf> {
        let mut command: Vec<PathBuf> = Vec::new();
        let songs: ReadDir = fs::read_dir(&directory).unwrap();
        for path in songs {
            let song = path.unwrap().path();
            match song.extension() {
                Some(ext) => match ext.to_str().unwrap() {
                    "flac" | "mp3" | "wav" | "ogg" => {
                        command.push(song)
                    }
                    _ => println!("Found extension {:?}, not playable", ext),
                },
                None => println!("No extension, skipping file"),
            }
        }

        

        return command;
    }

    pub fn set_volume(&self, volume: usize) {
        self.queue.set_volume(volume);
    }

    pub fn get_position(&self) -> usize {
        let current_position = self.queue.get_position();
        return current_position as usize;
    }

    pub fn get_playing(&self) -> bool {
        return self.queue.get_playing();
    }

    pub fn get_song_duration(&self) -> Duration {
        let final_duration;
        let duration_wrapped = self.queue.get_song_duration();
        match duration_wrapped {
            Some(duration) => {
                final_duration = duration;
            }
            _ => {
                final_duration = Duration::from_millis(0);
            }
        }
        return final_duration;
    }

    pub fn seek(&self, position: usize) {
        self.queue.seek_position(position);
    }

    pub fn current(&self) -> Option<Song>{
        return self.queue.get_current_song();
    }

    pub fn sort_songs(&self, songs: Vec<Song>) -> Vec<Song>{
        let mut sorted_command = Vec::new();
        let mut albums: HashMap<String, Vec<Song>> = HashMap::new();

        for song in songs{
            albums.entry(song.clone().album).or_insert(Vec::new()).push(song);
        }
        
        for (_title, album) in &mut albums {
            album.sort_by_key(|a| a.track_number);
        }

        let mut sorted_albums: Vec<(String, Vec<Song>)> = albums.into_iter().collect();
        sorted_albums.sort_by(|a,b| a.0.cmp(&b.0));

        for (_title, album) in sorted_albums{
            let album_paths: Vec<Song>  = album.iter().map(|s| s.clone()).collect();
            sorted_command.extend(album_paths);
        }

        return sorted_command;
    }
}

