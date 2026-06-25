use crossbeam_channel::{select_biased, Sender};
use rand::{prelude::*, rng};
use rodio::{Decoder, Player, Source};
use std::{fs, path::PathBuf, sync::Arc, sync::Mutex, thread, time::Duration};
use tauri::utils::config::Position;

#[derive(Clone)]
pub struct Queue {
    queue: Arc<Mutex<Vec<PathBuf>>>,
    stream_handle: Arc<rodio::MixerDeviceSink>,
    sink: Arc<Mutex<Player>>,
    up: Arc<Mutex<bool>>,
    index: Arc<Mutex<usize>>,
    upd_send: Sender<isize>,
    looping: Arc<Mutex<bool>>,
    ext_trigger: Arc<Mutex<bool>>,
    current_duration: Arc<Mutex<Option<Duration>>>,
}

impl Queue {
    pub fn new() -> Self {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let stream_handle = Arc::new(
            rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream"),
        );
        let sink = Arc::new(Mutex::new(Player::connect_new(stream_handle.mixer())));
        let up = Arc::new(Mutex::new(false));
        let looping = Arc::new(Mutex::new(false));
        let index = Arc::new(Mutex::new(0));
        let current_duration = Arc::new(Mutex::new(None));

        let ext_trigger = Arc::new(Mutex::new(true));

        let (dead_send, dead_recv) = crossbeam_channel::unbounded();
        let (upd_send, upd_recv) = crossbeam_channel::unbounded::<isize>();

        let monitor_sink = Arc::clone(&sink);
        let monitor_queue = Arc::clone(&queue);
        let monitor_up = Arc::clone(&up);
        let monitor_looping = Arc::clone(&looping);

        let monitor_duration = Arc::clone(&current_duration);

        let monitor_ext_trigger = Arc::clone(&ext_trigger);

        let monitor_index = Arc::clone(&index);

        let _monitor_thread = thread::spawn(move || {
            loop {
                let owned_index = Arc::clone(&monitor_index);
                select_biased! {
                    recv(upd_recv) -> msg => {
                        if let Ok(increment) = msg{
                            *monitor_ext_trigger.lock().unwrap() = false;
                            let new;
                            if increment > 0{
                                let current_idx: usize = *owned_index.lock().unwrap();
                                new = current_idx.saturating_add(increment.unsigned_abs());
                            } else if increment < 0{
                                let current_idx: usize = *owned_index.lock().unwrap();
                                new = current_idx.saturating_sub(increment.unsigned_abs());
                            } else {new = *owned_index.lock().unwrap()}
                            *owned_index.lock().unwrap() = new;
                            let song_wrapped = monitor_queue.lock().unwrap().get(*owned_index.lock().unwrap()).cloned();
                            match song_wrapped{
                                Some(song) => {
                                    monitor_play(&monitor_sink, &dead_send,song, &monitor_duration, &monitor_up);
                                },
                                _ => {*monitor_up.lock().unwrap() = false;}
                            }
                        }
                    }

                    recv(dead_recv) -> msg => {
                        if let Ok(_recieved) = msg{
                            if !*monitor_ext_trigger.lock().unwrap(){
                                 if !*monitor_looping.lock().unwrap(){
                                *owned_index.lock().unwrap() += 1;
                                }

                                let song_wrapped = monitor_queue.lock().unwrap().get(*owned_index.lock().unwrap()).cloned();
                                match song_wrapped{
                                    Some(song) => {
                                        monitor_play(&monitor_sink, &dead_send,song, &monitor_duration, &monitor_up);
                                    },
                                    _ => {*monitor_up.lock().unwrap() = false;}
                                }
                            }
                        }
                    }
                }
            }

            fn monitor_play(
                sink: &Arc<Mutex<Player>>,
                dead: &Sender<()>,
                path: PathBuf,
                duration_var: &Arc<Mutex<Option<Duration>>>,
                up: &Arc<Mutex<bool>>,
            ) {
                let owned_up = Arc::clone(up);
                let file = fs::File::open(path.clone()).unwrap();
                let source = Decoder::try_from(file).unwrap();
                let owned_duration_var = Arc::clone(duration_var);
                *owned_duration_var.lock().unwrap() = source.total_duration();
                let owned_sink = Arc::clone(sink);
                let owned_dead = dead.clone();
                let _player = thread::spawn(move || {
                    *owned_up.lock().unwrap() = true;
                    owned_sink.lock().unwrap().append(source);
                    owned_sink.lock().unwrap().play();
                    loop {
                        thread::sleep(std::time::Duration::from_millis(100));
                        if owned_sink.lock().unwrap().empty() {
                            break;
                        }
                    }
                    *owned_up.lock().unwrap() = false;
                    *owned_duration_var.lock().unwrap() = None;
                    owned_dead.send(()).unwrap();
                });
            }
        });

        return Queue {
            queue,
            stream_handle,
            sink,
            up,
            index,
            upd_send,
            looping,
            ext_trigger,
            current_duration,
        };
    }

    pub fn clear(&self) {
        *self.ext_trigger.lock().unwrap() = true;
        self.queue.lock().unwrap().clear();
        self.sink.lock().unwrap().stop();
        *self.index.lock().unwrap() = 0;
        *self.up.lock().unwrap() = false;
    }

    pub fn stop(&self) {
        let _ = &self.sink.lock().unwrap().stop();
    }

    pub fn add_songs(&self, song: Vec<PathBuf>) {
        {
            self.queue.lock().unwrap().extend(song);
        }
        if !*self.up.lock().unwrap() {
            *self.ext_trigger.lock().unwrap() = true;
            self.upd_send.send(0).unwrap();
            *self.ext_trigger.lock().unwrap() = true;
        }
    }

    pub fn pause(&self) {
        let _ = &self.sink.lock().unwrap().pause();
    }

    pub fn play(&self) {
        let _ = &self.sink.lock().unwrap().play();
    }

    pub fn skip_forward(&self) {
        self.pause();
        *self.ext_trigger.lock().unwrap() = true;
        self.stop();
        self.upd_send.send(1).unwrap();
    }

    pub fn skip_backward(&self) {
        self.pause();
        *self.ext_trigger.lock().unwrap() = true;
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

    pub fn get_current_song(&self) -> PathBuf {
        let index = *self.index.lock().unwrap();
        let queu_bind = self.queue.lock().unwrap();
        let path_wrapped = queu_bind.get(index);
        match path_wrapped {
            Some(path) => return path.to_path_buf(),
            _ => return PathBuf::new(),
        }
    }

    pub fn shuffle_queu(&self) {
        let mut current_queue = self.queue.lock().unwrap();
        let index = *self.index.lock().unwrap();
        let mut rng = rand::rng();
        current_queue[index + 1..].shuffle(&mut rng);
    }
}
