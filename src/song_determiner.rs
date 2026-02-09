use std::fs::File;

use babycat::constants::RESAMPLE_MODE_BABYCAT_LANCZOS;
use chromaprint::Chromaprint;
use babycat::{Waveform, WaveformArgs, batch};
use babycat::batch::waveforms_from_files;
use egui::IntoAtoms;
use egui::debug_text::print;
use egui::epaint::tessellator::Path;
use futures::FutureExt;
use genius_rust::song;
use tokio::process;
use chrono::Utc;
use std::path::PathBuf;




pub struct SongFingerprint {
    pub fingerprint: String,
    pub duration: f32,
}
impl SongFingerprint {
    pub fn decode_mp3(songs: Vec<PathBuf>) -> Option<Vec<Waveform>> {
        let mut waveforms = Vec::new();
        let songs_path_Strings:Vec<String> = songs.iter().map(|p| p.to_string_lossy().as_ref().to_string())
        .collect();
        let songs_slices: Vec<&str> = songs_path_Strings.iter().map(|s| s.as_str()).collect(); 

        let waveform_args  = WaveformArgs {
            convert_to_mono: true,
            resample_mode: RESAMPLE_MODE_BABYCAT_LANCZOS,
            ..Default::default()
        };

        let batch_args = Default::default();
    
        let batch = waveforms_from_files(
            &songs_slices,
            waveform_args,
            batch_args,
        );
        
    
        for named_result in batch {
            match &named_result.result {
            Ok(waveform) => {
                // Do further processing.               
                waveforms.push(waveform.clone());

            }
            Err(err) => {
                return None;
            }
        }
        



        }

        return Some(waveforms);


    }
    
    pub fn generate_fingerprint(songs: Vec<PathBuf>) -> Option<Vec<SongFingerprint>> {

        let processed_songs: Vec<Waveform> = Self::decode_mp3(songs)?;
        let mut song_fingerprints: Vec<SongFingerprint> = Vec::new();
        for processed_song in processed_songs {
            let samples_f32 = processed_song.to_interleaved_samples();
            let samples_i16: Vec<i16> = samples_f32
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();
            let mut chromaprint = Chromaprint::new();
            chromaprint.start(48000, 1);
            chromaprint.feed(&samples_i16);
            chromaprint.finish();
            let sample_rate = 48000; // u32
            let num_frames = processed_song.num_frames();   // usize
            let fingerprint = chromaprint.fingerprint().unwrap();
            let duration = num_frames as f64 / sample_rate as f64;
            song_fingerprints.push(SongFingerprint {
                fingerprint,
                duration: duration as f32
            });
        }

        return Some(song_fingerprints);
    }

    pub fn full_fingerprint(songs: Vec<PathBuf>) -> Vec<SongFingerprint> {
        let dt = Utc::now();
        let timestamp: i64 = dt.timestamp();
        let song_fingerprints = Self::generate_fingerprint(songs).unwrap();
        return song_fingerprints;
        let dt = Utc::now();
        let timestamp: i64 = dt.timestamp();
    }
}


