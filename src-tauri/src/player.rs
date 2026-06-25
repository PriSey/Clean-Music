use std::{collections::HashMap, fs, fs::ReadDir, path::PathBuf, time::Duration};
mod queue;
use lofty::{file::TaggedFile, prelude::*, probe::Probe, read_from_path};
use queue::Queue;
use tokio::fs::File;

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
            4 => self.queue.clear(),
            5 => self.queue.loop_song(),
            6 => self.queue.loop_stop(),
            _ => println!("How did you get here?"),
        }
    }

    pub fn play_song(&self, song: PathBuf) {
        self.queue.clear();
        self.queue.add_songs(self.load_song(song));
    }

    pub fn queue_song(&self, song: PathBuf) {
        self.queue.add_songs(self.load_song(song));
    }

    pub fn play_dir(&self, dir: PathBuf) {
        self.queue.clear();
        let dir_vec = self.load_dir(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn queue_dir(&self, dir: PathBuf) {
        let dir_vec = self.load_dir(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn load_song(&self, song: PathBuf) -> Vec<PathBuf> {
        let mut command = Vec::new();
        command.push(song);
        return command;
    }

    pub fn load_dir(&self, directory: PathBuf) -> Vec<PathBuf> {
        let mut command: Vec<PathBuf> = Vec::new();
        let songs: ReadDir = fs::read_dir(&directory).unwrap();
        let mut track_id = fs::read_dir(&directory).unwrap().count() as u32;
        let mut albums: HashMap<String, HashMap<PathBuf, u32>> = HashMap::new();
        for path in songs {
            let song = path.unwrap().path();
            match song.extension() {
                Some(ext) => match ext.to_str().unwrap() {
                    "flac" | "mp3" | "wav" | "ogg" => {
                        let tagged_file = read_from_path(song.clone()).unwrap();
                        let tags_wrapped = match tagged_file.primary_tag() {
                            Some(primary_tag) => Some(primary_tag),
                            None => tagged_file.first_tag(),
                        };
                        match tags_wrapped {
                            Some(tags) => {
                                let track = match tags.track() {
                                    Some(track) => track,
                                    None => {
                                        track_id += 1;
                                        track_id
                                    }
                                };
                                albums
                                    .entry(tags.album().unwrap_or_default().to_string())
                                    .or_insert(HashMap::new())
                                    .insert(song, track);
                            }
                            None => {
                                track_id += 1;
                                let track = track_id;
                                albums
                                    .entry("None".to_string())
                                    .or_insert(HashMap::new())
                                    .insert(song, track);
                            }
                        }
                    }
                    _ => println!("Found extension {:?}, not playable", ext),
                },
                None => println!("No extension, skipping file"),
            }
        }

        let mut sorted_albums: Vec<(String, Vec<PathBuf>)> = Vec::new();
        for (album, songs_sorted) in albums {
            let mut count_vec: Vec<(&PathBuf, &u32)> = songs_sorted.iter().collect();
            count_vec.sort_by(|a, b| a.1.cmp(b.1));
            sorted_albums.push((album, count_vec.iter().map(|x| x.0.clone()).collect()));
        }

        sorted_albums.sort_by(|a, b| a.1.cmp(&b.1));

        for (_album, sorted_album) in sorted_albums {
            command.extend(sorted_album)
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

    pub fn current(&self) -> PathBuf {
        return self.queue.get_current_song();
    }
}
