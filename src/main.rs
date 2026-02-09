
use std::path::PathBuf;
use std::{fs::File};
use std::thread;
use egui::epaint::tessellator::Path;
use futures::io::BufReader;
use id3::{Tag, TagLike};
use std::sync::mpsc::{self, Sender};
use rodio::{Decoder, Sink, math};
use egui_file_dialog::FileDialog;
use eframe;
use std::sync::Arc;
use genius_rust::Genius;
use dotenv;
use tokio::{self, fs};
use chartlyrics::Client;
mod song_data;
mod song_determiner;
use tokio::runtime::Runtime;
use id3_image::embed_image;
use std::env::current_dir;

fn main() -> Result<(), eframe::Error> {
    dotenv::dotenv().ok();
    let genius = Genius::new(std::env::var("TOKEN").expect("TOKEN not set"));

    let (send_sink,rec_sink) = mpsc::channel();
    let (send_song,rec_song) = mpsc::channel::<Vec<PathBuf>>();
    // let song: Vec<&str> = vec!["./songs/test3.mp3","./songs/Test4.mp3","./songs/test5.mp3","./songs/test2.mp3","./songs/test6.mp3"];

    // rt.block_on(song_data::SongData::process_songs(song, "PUfuD1toVu"));

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
            picking: false,
            genius: genius,
            lyrics: String::new()
            }))))
    
}

struct Song {
    path: PathBuf,
    track: Option<u32>,
}

pub struct MessageWindow{
    message: String
}

impl eframe::App for MessageWindow{
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
         egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(&self.message)
         });
    }
}
impl MessageWindow{
    pub fn pop_up(self) -> Result<(), eframe::Error>{
        let options = eframe::NativeOptions::default();

            eframe::run_native("No Result Found", options, Box::new(|_cc| 
        Ok(Box::new(MessageWindow { 
            message:self.message
            }))))
    }
    fn new(message: String) -> Self{
        Self{
            message: message
        }
    }
}


pub struct MyApp{
    sender: Sender<usize>,
    path: String,
    song_sender: Sender<Vec<PathBuf>>,
    song_paths: Vec<PathBuf>,
    file_dialog: FileDialog,
    picked_file: Option<PathBuf>,
    picking: bool,
    genius: Genius,
    lyrics: String
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

                    ui.heading("Add and Process music");

                    ui.end_row();

                    if ui.button("Pick File").clicked(){
                        self.picking = true;
                        self.file_dialog.pick_multiple();
                    }

                    if self.picking{self.file_dialog.update(ctx);}
                    if let Some(path) = self.file_dialog.take_picked_multiple() {
                        self.song_paths.clear();
                        self.picking = false;
                        let paths = Some(path).clone().unwrap();
                        //println!("{:?}",paths);
                        let _task = thread::spawn(move || {
                            println!("Thread Spawned");
                            // let mut path_splits: Vec<Vec<PathBuf>> = Vec::new();
                            // let mut split: Vec<PathBuf> = Vec::new();
                            // let mut song_data: Vec<song_data::SongData> = Vec::new();

                            for song in paths{
                                thread::spawn(move || {
                                    let rt = Runtime::new().unwrap();
                                    println!("Processing song: {:?}", song);
                                    let song_data = &rt.block_on(song_data::SongData::process_songs(vec![song.clone()], "PUfuD1toVu"))[0];
                                
                                if song_data.title == "Unknown".to_string(){
                                    let message = format!("FAILED processing {:?}" , song).to_string();
                                    println!("{}", message);
                                    // let pop_up = MessageWindow::new(message);
                                    // pop_up.pop_up().unwrap()
                                } else {
                                    println!("{:?}",song);
                                    let song_path = PathBuf::from(song);
                                    let new_name = PathBuf::from(format!("./Stored_Songs/{}.mp3", song_data.title.replace("/","")));
                                    rt.block_on(fs::copy(song_path,&new_name)).unwrap();
                                    let mut tag = Tag::read_from_path(&new_name).unwrap();

                                    tag.set_title(&song_data.title);
                                    println!("{}",&song_data.title);
                                    tag.set_artist(&song_data.artist);
                                    println!("{}",&song_data.artist);
                                    tag.set_album(&song_data.album);
                                    println!("{}",&song_data.album);

                                    tag.write_to_path(&new_name, id3::Version::Id3v24).unwrap();
                                    if let Some(image_path) = song_data.image.clone(){
                                        match embed_image(&new_name, &image_path) {
                                            Ok(_) => (),
                                            Err(_e) => println!("Image failed to read, skipping")
                                        }
                                        
                                    } else {
                                        println!("No image, skipping image processing")
                                    }
                                }


                                });
                            }
                        });

                    

                    }

                    

                  
            });
        });
              

    });
        egui::SidePanel::right("sidebar").resizable(true).show(ctx, |ui| {
        ui.heading("Lyrics - May be innacurate");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.label(&self.lyrics);

            ui.set_min_width(300.0);
        });
    });
}

}




// fn get_song(directory: &PathBuf) -> Vec<PathBuf>{
//     if directory.extension().and_then(OsStr::to_str) == Some("mp3") {
//         let mut output = Vec::new();
//         output.push(directory.clone());
//         return output;
//     } else if directory.is_dir() {
//         let mut output = Vec::new();
//         let mut songs: Vec<Song> = fs::read_dir(directory).unwrap()
//             .filter_map(|entry| {
//                 let entry = entry.ok()?;
//                 let path = entry.path();
//                 if path.extension()? == "mp3" {
//                     let track = get_track_number(&path);
//                     Some(Song { path, track })
//                 } else {
//                     None
//                 }
//             })
//             .collect();

//         songs.sort_by_key(|s| s.track.unwrap_or(u32::MAX));

//         let song_paths: Vec<PathBuf> =  songs.iter().map(|s| s.path.clone()).collect();    


//         for song in song_paths {
//             let song_path: PathBuf = song;
//             if song_path.extension().and_then(OsStr::to_str) == Some("mp3") {
//                 output.push(song_path);
//             }   
//         }
//         return output.clone();
//         } 
//         else {
//         let output = Vec::new();

//         return output;
// }
// }
fn _get_track_number(path: &PathBuf) -> Option<u32> {
    let tag = Tag::read_from_path(path).ok()?;
    tag.track()
}

async fn _get_lyrics(name: &str, artist: &str) -> String{

    let client = Client::new().await.unwrap();
    let result = client.search_lyric_direct(&name.to_lowercase(),&artist.to_lowercase()).await.unwrap();
    //println!("{}", result.lyrics); 

    return result.lyrics;
}

pub fn remove_whites(mut string:String) -> String{
    string.retain(|c| !c.is_whitespace());
    return string;
}

fn process_mp3(path: &PathBuf, name: &str, artist_credit: &str, album: &str, file: PathBuf) {

}