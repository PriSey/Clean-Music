use crossbeam_channel::{select_biased, Sender};
use rand::{prelude::*};
use rodio::{Decoder, Player, Source, mixer};
use crate::song::Song;
use std::{fs, path::PathBuf, sync::Arc, sync::Mutex, thread, time::Duration, option::Option};

#[derive(Clone)]
pub struct Queue {
    queue: Arc<Mutex<Vec<Song>>>,
    sink: Arc<Mutex<Player>>,
    up: Arc<Mutex<bool>>,
    index: Arc<Mutex<usize>>,
    upd_send: Sender<isize>,
    looping: Arc<Mutex<bool>>,
    kill_send: Sender<()>,
    current_duration: Arc<Mutex<Option<Duration>>>,
}

impl Queue {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(Vec::<Song>::new()));
        let stream_handle = Arc::new(
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream"),
        );
        let sink = Arc::new(Mutex::new(Player::connect_new(stream_handle.mixer())));
        let up = Arc::new(Mutex::new(false));
        let looping = Arc::new(Mutex::new(false));
        let index = Arc::new(Mutex::new(0));
        let current_duration = Arc::new(Mutex::new(None));

        let (dead_send, dead_recv) = crossbeam_channel::unbounded();
        let (upd_send, upd_recv) = crossbeam_channel::unbounded::<isize>();
        let (kill_send, kill_recv) = crossbeam_channel::unbounded();

        let monitor_sink = Arc::clone(&sink);
        let monitor_queue = Arc::clone(&queue);
        let monitor_up = Arc::clone(&up);
        let monitor_looping = Arc::clone(&looping);
        let monitor_kill_request = Arc::new(Mutex::new(false));
        let monitor_stream_handle = Arc::clone(&stream_handle);

        let monitor_duration = Arc::clone(&current_duration);

        let monitor_index = Arc::clone(&index);

        let _monitor_thread = thread::spawn(move || {
            let mut current_player_thread: Option<std::thread::JoinHandle<()>> = None;
            loop {
                let owned_index = Arc::clone(&monitor_index);
                select_biased! {

                    recv(kill_recv) -> msg => {
                      if let Ok(_recieved) = msg{
                        *monitor_kill_request.lock().unwrap() = true;
                        monitor_sink.lock().unwrap().stop();
                        match current_player_thread.take(){
                            Some(p_thread) => {_ = p_thread.join();},
                            _ => {}
                        }
                        *monitor_kill_request.lock().unwrap() = false;
                        current_player_thread = None;

                      }
                    }

                    recv(upd_recv) -> msg => {
                        if let Ok(increment) = msg{
                            let last = monitor_queue.lock().unwrap().len().saturating_sub(1);
                            let new;
                            if increment > 0{
                                let current_idx: usize = *owned_index.lock().unwrap();
                                new = current_idx.saturating_add(increment.unsigned_abs()).min(last);
                            } else if increment < 0{
                                let current_idx: usize = *owned_index.lock().unwrap();
                                new = current_idx.saturating_sub(increment.unsigned_abs());
                            } else{
                              if*monitor_up.lock().unwrap() == false{
                                new = *owned_index.lock().unwrap()
                              } else {
                                continue;
                              }
                            }
                            *owned_index.lock().unwrap() = new;
                            let song_wrapped = monitor_queue.lock().unwrap().get(*owned_index.lock().unwrap()).cloned();
                            match song_wrapped{
                                Some(song) => {
                                    match current_player_thread.take(){
                                      Some(p_thread) => {
                                        if p_thread.is_finished(){
                                          current_player_thread = Some(monitor_play(monitor_stream_handle.mixer(),&monitor_sink, &dead_send, song.path, &monitor_duration, &monitor_up, &monitor_kill_request));
                                        }
                                      },
                                      _ => {current_player_thread = Some(monitor_play(monitor_stream_handle.mixer(),&monitor_sink, &dead_send, song.path, &monitor_duration, &monitor_up, &monitor_kill_request))}
                                    }
                                },
                                _ => {*monitor_up.lock().unwrap() = false;}
                            }
                        }
                    }

                    recv(dead_recv) -> msg => {
                        if let Ok(_recieved) = msg{
                          if !*monitor_looping.lock().unwrap(){
                              *owned_index.lock().unwrap() += 1;
                          }

                          let song_wrapped = monitor_queue.lock().unwrap().get(*owned_index.lock().unwrap()).cloned();
                          match song_wrapped{
                              Some(song) => {
                                match current_player_thread.take(){
                                  Some(p_thread) => {
                                      if p_thread.is_finished(){
                                        current_player_thread = Some(monitor_play(monitor_stream_handle.mixer(),&monitor_sink, &dead_send, song.path, &monitor_duration, &monitor_up, &monitor_kill_request));
                                    }
                                  },
                                  _ => {current_player_thread = Some(monitor_play(monitor_stream_handle.mixer(),&monitor_sink, &dead_send, song.path, &monitor_duration, &monitor_up, &monitor_kill_request))}
                                }
                              },
                              _ => {*monitor_up.lock().unwrap() = false;}
                          }      
                        }
                    }

                }
            }

            fn monitor_play(
                stream_handle: &mixer::Mixer,
                sink: &Arc<Mutex<Player>>,
                dead: &Sender<()>,
                path: PathBuf,
                duration_var: &Arc<Mutex<Option<Duration>>>,
                up: &Arc<Mutex<bool>>,
                kill_request: &Arc<Mutex<bool>>
            ) -> std::thread::JoinHandle<()>{
                let volume = sink.lock().unwrap().volume();
                let new = Player::connect_new(stream_handle);
                new.set_volume(volume);
                *sink.lock().unwrap() = new;
                let owned_up = Arc::clone(up);
                let file = fs::File::open(path.clone()).unwrap();
                let source = Decoder::try_from(file).unwrap();
                let owned_duration_var = Arc::clone(duration_var);
                *owned_duration_var.lock().unwrap() = source.total_duration();
                let owned_sink = Arc::clone(sink);
                let owned_kill_request = Arc::clone(kill_request);
                let owned_dead = dead.clone();
                *up.lock().unwrap() = true;
                let player = thread::spawn(move || {
                    let mut kill = false;
                    owned_sink.lock().unwrap().play();
                    owned_sink.lock().unwrap().append(source);
                    loop {
                        thread::sleep(std::time::Duration::from_millis(10));
                        if *owned_kill_request.lock().unwrap(){
                          kill = true;
                          break;
                        }
                        if owned_sink.lock().unwrap().empty() {
                            break;
                        }
                    }

                    *owned_up.lock().unwrap() = false;
                    *owned_duration_var.lock().unwrap() = None;                    
                    if kill == false{
                        owned_dead.send(()).unwrap();
                    }
                });
                return player;
            }
        });

        return Queue {
            queue,
            sink,
            up,
            index,
            upd_send,
            looping,
            kill_send,
            current_duration,
        };
    }

    pub fn stop(&self) {
      self.kill_send.send(()).unwrap();
    }

    pub fn reset(&self) {
        self.queue.lock().unwrap().clear();
        self.stop();
        *self.index.lock().unwrap() = 0;
        *self.up.lock().unwrap() = false;
    }

    pub fn clear(&self){
      self.queue.lock().unwrap().clear();
      self.stop();
      *self.up.lock().unwrap() = false;
    }
    
    pub fn add_songs(&self, song: Vec<Song>) {
      self.queue.lock().unwrap().extend(song);
      self.upd_send.send(0).unwrap();
    }

    pub fn go_to(&self, position:usize){
      *self.index.lock().unwrap() = position;
    }

    pub fn pause(&self) {
        let _ = &self.sink.lock().unwrap().pause();
    }

    pub fn play(&self) {
        let _ = &self.sink.lock().unwrap().play();
    }

    pub fn skip_forward(&self) {
        self.pause();
        self.stop();
        self.upd_send.send(1).unwrap();
    }

    pub fn skip_backward(&self) {
        self.pause();
        self.stop();
        self.upd_send.send(-1).unwrap();
    }

    pub fn loop_song(&self) {
        *self.looping.lock().unwrap() = true;
    }

    pub fn loop_stop(&self) {
        *self.looping.lock().unwrap() = false;
    }

    pub fn set_volume(&self, volume: usize) {
        let volume_adjusted = (volume as f32 / 100.0).powf(3.0);
        self.sink.lock().unwrap().set_volume(volume_adjusted);
    }

    pub fn get_position(&self) -> u128 {
        let current_pos = self.sink.lock().unwrap().get_pos();
        return Duration::as_millis(&current_pos);
    }

    pub fn seek_position(&self, position: usize) {
        let seek_duration = Duration::from_millis(position as u64);
        if let Err(e) = self.sink.lock().unwrap().try_seek(seek_duration) {
            println!("Seeking not supported for this file. {}", e);
        }
    }

    pub fn get_playing(&self) -> bool {
        return *self.up.lock().unwrap();
    }

    pub fn get_song_duration(&self) -> Option<Duration> {
        return *self.current_duration.lock().unwrap();
    }

    pub fn get_current_song(&self) -> Option<Song> {
        let index = *self.index.lock().unwrap();
        let queu_bind = self.queue.lock().unwrap();
        let song = queu_bind.get(index).cloned();
        return song;
    }

    pub fn shuffle_queu(&self) {
        let mut current_queue = self.queue.lock().unwrap();
        let index = *self.index.lock().unwrap();
        let mut rng = rand::rng();
        current_queue[index + 1..].shuffle(&mut rng);
    }
}
