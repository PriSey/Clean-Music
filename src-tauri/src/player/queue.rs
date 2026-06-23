use rodio::{Decoder, Player};
use std::{fs, path::PathBuf, thread, sync::Mutex,sync::Arc, };
use crossbeam_channel::{Sender,select};


#[derive(Clone)]
pub struct Queue{
    queue: Arc<Mutex<Vec<PathBuf>>>,
    sink: Arc<Mutex<Player>>,
    queue_history: Arc<Mutex<Vec<PathBuf>>>,
    up: Arc<Mutex<bool>>,
    current: Arc<Mutex<Option<PathBuf>>>,
    update_queue: Sender<()>,
    looping: Arc<Mutex<bool>>
}

impl Queue{

    pub fn new() -> Self{
        let queue = Arc::new(Mutex::new(Vec::new()));
        let queue_history = Arc::new(Mutex::new(Vec::new()));
        let stream_handle = rodio::DeviceSinkBuilder::open_default_sink().expect("open default audio stream");
        let sink = Arc::new(Mutex::new(Player::connect_new(stream_handle.mixer())));
        let up = Arc::new(Mutex::new(false));
        let current = Arc::new(Mutex::new(Option::None));
        let looping = Arc::new(Mutex::new(false));

        let (upd_send, upd_recv) = crossbeam_channel::unbounded();
        let main_send = upd_send.clone();
        

        let monitor_sink = Arc::clone(&sink);
        let monitor_queue = Arc::clone(&queue);
        let monitor_up = Arc::clone(&up);
        let monitor_looping = Arc::clone(&looping);
        

        let monitor_current = Arc::clone(&current);
        let monitor_history= Arc::clone(&queue_history);

        let _monitor_thread = thread::spawn(move || {
            println!("Monitor Running");
            loop{
                select!{
                    recv(upd_recv) -> msg => {
                        if let Ok(_recieved) = msg{
                            let song_wrapped;
                            if *monitor_looping.lock().unwrap(){
                                song_wrapped = monitor_history.lock().unwrap().last().cloned();
                              
                            } else {
                                song_wrapped = monitor_queue.lock().unwrap().pop();
                            }   
                            match song_wrapped{
                                Some(song) => {
                                    monitor_play(&monitor_sink, &upd_send,song, &monitor_current, &monitor_history); 
                                    *monitor_up.lock().unwrap() = true;
                                },
                                _ => {*monitor_up.lock().unwrap() = false}
                            }
                        }                            
                    }
                }
            }
        


            fn monitor_play(sink: &Arc<Mutex<Player>>, dead: &Sender<()>, path: PathBuf, current: &Arc<Mutex<Option<PathBuf>>>, history: &Arc<Mutex<Vec<PathBuf>>>){
                *current.lock().unwrap() = Some(path.clone());
                let file = fs::File::open(path.clone()).unwrap();
                let source = Decoder::try_from(file).unwrap();
                let owned_sink = Arc::clone(sink);
                let owned_dead = dead.clone();
                let owned_current = Arc::clone(current);
                let owned_history = Arc::clone(history);
                let _player = thread::spawn(move || {
                    owned_sink.lock().unwrap().play();
                    owned_sink.lock().unwrap().append(source);
                    loop{
                        thread::sleep(std::time::Duration::from_millis(100));
                        if owned_sink.lock().unwrap().empty(){
                            break;
                        }
                    }
                    *owned_current.lock().unwrap() = None;
                    owned_history.lock().unwrap().push(path.clone());
                    owned_dead.send(()).unwrap();
                });
            }
        });

        return Queue { queue, sink, queue_history, up, current, update_queue: main_send.clone(), looping };
    }
    
    pub fn clear(&self){
        self.queue.lock().unwrap().clear();
    }

    pub fn stop(&self){
        &self.sink.lock().unwrap().stop();
    }

    pub fn add_songs(&self, song: Vec<PathBuf>){
        self.queue.lock().unwrap().extend(song);
        if *self.up.lock().unwrap(){
            self.update_queue.send(());
        }
    }

    pub fn pause(&self){
        &self.sink.lock().unwrap().pause();
    }

    pub fn play(&self){
        &self.sink.lock().unwrap().play();
    }

    pub fn skip_forward(&self){
        self.pause();
        let skipped_wrapped = self.queue.lock().unwrap().pop();
        match skipped_wrapped{
            Some(skipped) => {
                self.queue_history.lock().unwrap().push(skipped);
            },
            _ => {}
        }
        self.stop();
    }

    pub fn skip_backward(&self){
        self.pause();
        let skipped_wrapped = self.queue_history.lock().unwrap().pop();
        match skipped_wrapped{
            Some(skipped) => {
                self.queue.lock().unwrap().insert(0,skipped);
            },
            _ => {}
        }
        self.stop();
    }

    pub fn loop_song(&self){
        *self.looping.lock().unwrap() = true;
    }

    pub fn loop_stop(&self){
        *self.looping.lock().unwrap() = false;
    }

    pub fn set_volume(&self, volume:usize){
        let volume_adjusted = (volume as f32 / 100.0).powf(3.0);
        self.sink.lock().unwrap().set_volume(volume_adjusted);
    }
}