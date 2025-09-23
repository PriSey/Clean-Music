
use std::ffi::OsStr;
use std::path::PathBuf;
use std::{fs::File, fs};
use std::thread;
use id3::{Tag, TagLike};
use std::sync::mpsc::{self, Sender};
use rodio::{Decoder, Sink};
use egui_file_dialog::FileDialog;
use eframe;
use std::sync::Arc;

fn main() -> Result<(), eframe::Error> {
    
    let (send_sink,rec_sink) = mpsc::channel();
    let (send_song,rec_song) = mpsc::channel::<Vec<PathBuf>>();

    let options = eframe::NativeOptions::default();

    let _player = thread::spawn(move ||{
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        let sink = rodio::Sink::connect_new(&stream_handle.mixer());

        

        loop {  
                if let Ok(recieved) = rec_sink.try_recv(){
                    println!("from: {:?}",recieved);
                    match recieved{
                        0 => sink.pause(),
                        1 => sink.play(),
                        2 => sink.skip_one(),
                        3 => sink.clear(),
                        _ => println!("TODO")
                    }
                };
                if let Ok(songs) = rec_song.try_recv(){
                    for song in 0..songs.len(){
                        sink.play();
                        println!("{:?}",songs.get(song));
                        add_song(songs.get(song).unwrap().clone(), &sink);
                    }
                //     println!("{:?}",song);
                //     add_song(song, &sink);   
                }          


        }

        fn add_song(path:PathBuf, sink:&Sink){
            let file = File::open(path).unwrap();
            let source = Decoder::try_from(file).unwrap();
            sink.append(source);
        }
    });

    eframe::run_native("FUCKING GUI", options, Box::new(|_cc| 
        Ok(Box::new(MyApp { 
            sender: send_sink, 
            path: String::new(), 
            song_sender: send_song, 
            song_paths:Vec::new(),
            file_dialog: FileDialog::new().add_file_filter(
        "MP3 filters",
        Arc::new(|p| p.extension().unwrap_or_default() == "mp3"),
                ).default_file_filter("MP3 filters"),
            picked_file: None,
            picking: false
            }))))
    
}

struct Song {
    path: PathBuf,
    track: Option<u32>,
}

pub struct MyApp{
    sender: Sender<usize>,
    path: String,
    song_sender: Sender<Vec<PathBuf>>,
    song_paths: Vec<PathBuf>,
    file_dialog: FileDialog,
    picked_file: Option<PathBuf>,
    picking: bool
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("Hello").show(ui, |ui|{
                    let mut pickedFile: Option<PathBuf>;
                    ui.heading("Music App");
                    ui.end_row();
                    let buttons: [egui::Response; 4] = [ui.button("Pause"),ui.button("Play"),ui.button("Skip"),ui.button("Clear")];
                    for i in 0..buttons.len(){
                        if buttons[i].clicked() {
                            self.sender.send(i).unwrap();
                        }
                    }
                    ui.end_row();

                    ui.heading("All your music!");

                    ui.end_row();
                    
                    if ui.button("Pick Folder (Playlist)").clicked(){
                        self.picking = true;
                        self.file_dialog.pick_directory();
                    }

                    if ui.button("Pick File").clicked(){
                        self.picking = true;
                        self.file_dialog.pick_file();
                    }

                    if self.picking{self.file_dialog.update(ctx);}
                    if let Some(path) = self.file_dialog.take_picked() {
                        self.song_paths.clear();
                        self.picking = false;
                        let paths = Some(path).clone().unwrap();
                        self.song_paths.push(paths.clone());
                        self.song_paths.extend(get_song(&paths));

                    }

                    

                    for directory in self.song_paths.clone(){
                        let director = directory.display().to_string();
                        let name = (director.split("/").last().unwrap()).split(".").next().unwrap();
                        if(directory.is_dir()){
                            ui.end_row();
                            ui.label("Folder/Playlist");
                            ui.end_row();
                            ui.label(name);
                            if ui.button("Play").clicked(){
                                self.sender.send(3).unwrap();
                                self.song_sender.send(get_song(&directory)).unwrap();
                            }
                            if ui.button("Queue").clicked(){
                                self.song_sender.send(get_song(&directory)).unwrap();
                            }
                            ui.end_row();
                            ui.label("----------------------------------------------------------");
                        } else {
                            ui.end_row();
                            ui.label(name);
                            if ui.button("Play").clicked(){
                                self.sender.send(3).unwrap();
                                self.song_sender.send(get_song(&directory)).unwrap();
                            }
                            if ui.button("Queue").clicked(){
                                self.song_sender.send(get_song(&directory)).unwrap();
                            }
                        }
                    }
                //}  
            });
        });
              

    });
}
    
}
fn get_song(directory: &PathBuf) -> Vec<PathBuf>{
    if directory.extension().and_then(OsStr::to_str) == Some("mp3") {
        let mut output = Vec::new();
        output.push(directory.clone());
        return output;
    } else if directory.is_dir() {
        let mut output = Vec::new();
        let mut songs: Vec<Song> = fs::read_dir(directory).unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension()? == "mp3" {
                    let track = get_track_number(&path);
                    Some(Song { path, track })
                } else {
                    None
                }
            })
            .collect();

        songs.sort_by_key(|s| s.track.unwrap_or(u32::MAX));

        let song_paths: Vec<PathBuf> =  songs.iter().map(|s| s.path.clone()).collect();    


        for song in song_paths {
            let song_path: PathBuf = song;
            if song_path.extension().and_then(OsStr::to_str) == Some("mp3") {
                output.push(song_path);
            }   
        }
        return output.clone();
        } 
        else {
        let output = Vec::new();

        return output;
}
}
fn get_track_number(path: &PathBuf) -> Option<u32> {
    let tag = Tag::read_from_path(path).ok()?;
    tag.track()
}

