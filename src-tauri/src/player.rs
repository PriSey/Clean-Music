use rodio::{Decoder, Player as Sink, math};
use std::{fs, path::PathBuf, thread, fs::ReadDir};
use crossbeam_channel::{Sender,select};
mod queue;
use queue::Queue;

#[derive (Clone)]
pub struct Player{
    queue: Queue

}

impl Player {
    pub fn new() -> Self{
        let queue = Queue::new();

        return Player{queue};
    }

    pub fn send_command(&self, command:usize){
        match command{
            0 => self.queue.pause(),
            1 => self.queue.play(),
            2 => self.queue.skip_forward(),
            3 => self.queue.skip_backward(),
            4 => self.queue.clear(),
            _ => println!("How did you get here?")
        }
    }

    
    pub fn play_song(&self, song:PathBuf){
        self.queue.clear();
        self.queue.add_songs(self.load_song(song));
    }

    pub fn queue_song(&self, song:PathBuf){

        self.queue.add_songs(self.load_song(song));
    }

    pub fn play_dir(&self, dir:PathBuf){
        self.queue.clear();
        let dir_vec = self.load_dir(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn queue_dir(&self, dir:PathBuf){
        let dir_vec = self.load_dir(dir);
        self.queue.add_songs(dir_vec)
    }

    pub fn load_song(&self, song:PathBuf) -> Vec<PathBuf>{
        let mut command = Vec::new();
        command.push(song);
        return command;
    }

    pub fn load_dir(&self, directory:PathBuf) -> Vec<PathBuf>{
        let mut command: Vec<PathBuf> = Vec::new();
        let songs: ReadDir = fs::read_dir(directory).unwrap();
        for path in songs{
            let song = path.unwrap().path();
            match song.extension() {
                Some(ext) => {
                    match ext.to_str().unwrap() {
                         "flac" => command.push(song),
                         "mp3" => command.push(song),
                         "wav" => command.push(song),
                         "ogg" => command.push(song),
                        _ => println!("Found extension {:?}, not playable",ext)
                    }
                },
                None => println!("No extension, skipping file")
            }
        }

        return command;
    }

    pub fn set_volume(&self, volume:usize){
        self.queue.set_volume(volume);
    }
}
