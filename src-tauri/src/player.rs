use rodio::{Decoder, Player as Sink, queue};
use std::{fs, path::PathBuf, thread, fs::ReadDir};
use crossbeam_channel::{Sender,select};

#[derive (Clone)]
pub struct Player{
    send_sink: Sender<usize>,
    send_song: Sender<Vec<PathBuf>>,
    send_vol: Sender<usize>,
    queue: Vec<PathBuf>

}

impl Player {
    pub fn new() -> Self{
        let (send_sink, recv_sink) = crossbeam_channel::unbounded::<usize>();
        let (send_song,recv_song) =  crossbeam_channel::unbounded::<Vec<PathBuf>>();
        let (send_vol, recv_vol) = crossbeam_channel::unbounded::<usize>();
        let queue: Vec<PathBuf> = Vec::new();

        let _player_thread = thread::spawn(move||{


            let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
            let sink = Sink::connect_new(stream_handle.mixer());

            loop{               
                select! {
                    recv(recv_sink) -> msg => {
                        if let Ok(recieved) = msg{
                            match recieved{
                                0 => sink.pause(),
                                1 => sink.play(),
                                2 => sink.skip_one(),
                                3 => sink.clear(),
                                _ => ()
                            }

                        }
                },
                recv(recv_song) -> msg => {
                    if let Ok(song) = msg
                        {
                            sink.play();
                            add_song(song.get(0).unwrap().to_path_buf(), &sink);
                              
                        }
                    }
                recv(recv_vol) -> msg => {
                    if let Ok(volume) = msg{
                        let volume_adjusted = (volume as f32 / 100.0).powf(3.0);
                        sink.set_volume(volume_adjusted);
                    }
                }      

                }
            }
            fn add_song(path:PathBuf, sink:&Sink){
                let file = fs::File::open(path).unwrap();
                let source = Decoder::try_from(file).unwrap();
                sink.append(source);
            }
        });
        
        Player {send_sink, send_song, send_vol, queue }

    }

    pub fn send_command(&self, command:usize){
        self.send_sink.send(command).unwrap();
    }


    
    pub fn play_song(&self, song:PathBuf){
        self.send_command(3);
        let song_vec= self.load_song(song);
        self.send_song.send(song_vec).unwrap();
    }

    pub fn queue_song(&self, song:PathBuf){
        let song_vec = self.load_song(song);
        self.send_song.send(song_vec).unwrap();
    }

    pub fn play_dir(&self, dir:PathBuf){
        self.send_command(3);
        let dir_vec = self.load_dir(dir);
        self.send_song.send(dir_vec).unwrap();
    }

    pub fn queue_dir(&self, dir:PathBuf){
        let dir_vec = self.load_dir(dir);
        self.send_song.send(dir_vec).unwrap();
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
        self.send_vol.send(volume).unwrap();
    }
}
