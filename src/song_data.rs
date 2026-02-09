
use futures::future::join_all;
use musicbrainz_rs_nova::entity::artist::{Artist};
use musicbrainz_rs_nova::entity::CoverartResponse;
use musicbrainz_rs_nova::entity::release_group::{ReleaseGroupPrimaryType};
use reqwest::Client;
use std::collections::HashMap;
use serde_json::Value;
use crate::song_determiner::SongFingerprint;
use musicbrainz_rs_nova::{Fetch, entity::recording::Recording, entity::release::Release, FetchCoverart,};
use musicbrainz_rs_nova::entity::release::{ReleaseStatus};
use std::path::PathBuf;
use futures::stream::FuturesOrdered;
use futures::Future;
use std::pin::Pin;
use tokio;
use infer;
pub struct SongData {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f32,
    pub image: Option<PathBuf>,
}
impl SongData{
    pub async fn process_songs(songs: Vec<PathBuf>, api_key: &str) -> Vec<SongData> {

        musicbrainz_rs_nova::config::set_user_agent("Clean_Music/1.0.1a (https://github.com/PriSey/Clean-Music");

        let mut tasks = Vec::new();
        let client = Client::new();
        let fingerprints = SongFingerprint::full_fingerprint(songs);
        let api_key_owned = api_key.to_string();
        let client_owned = client.clone();
        let mut stream: FuturesOrdered<Pin<Box<dyn Future<Output = SongData> + Send>>> = FuturesOrdered::new();

        for fingerprint in fingerprints {
            let duration = fingerprint.duration.round() as u32;
            let fingerprint_f = fingerprint.fingerprint;
            let api_key_clone = api_key_owned.clone();
            let client_clone = client_owned.clone();
            let task = tokio::spawn(async move {
                let mut song_info: (String,String,String,Option<PathBuf>) = ("Unknown".to_string(),"Unknown".to_string(),"Unknown".to_string(),None);
                let mut params: HashMap<String, String> = HashMap::new();
                params.insert("client".to_string(), api_key_clone.to_string());
                params.insert("duration".to_string(), duration.to_string());
                params.insert("fingerprint".to_string(), fingerprint_f);
                params.insert(
                    "meta".to_string(),
                    "recordings".to_string(),
                );
                let res = client_clone
                .post("https://api.acoustid.org/v2/lookup")
                .form(&params)
                .send().await.expect("AcoustID request failed")
                .text().await.expect("Failed to parse response text");

            let musicbrainz_id = Self::get_musicbrainz_id(&res,duration).unwrap();

            for i in musicbrainz_id.clone(){
                let song_info_inner = Self::fetch_recording_info(&i).await.unwrap();
                let artist_albums_offial = Self::fetch_releases_from_artist(&song_info_inner.5).await.unwrap();
                if song_info_inner.3 {
                    if artist_albums_offial.0.contains(&song_info_inner.2.get(0).unwrap().title) || artist_albums_offial.1.contains(&song_info_inner.2.get(0).unwrap().title){
                        song_info.0 = song_info_inner.0.clone();
                        song_info.1 = song_info_inner.1.clone();
                        song_info.2 = song_info_inner.2.get(0).unwrap().title.clone();
                         let coverart_url = Self::fetch_image(song_info_inner.2.get(0).unwrap().clone()).await;
                         if coverart_url.is_some(){
                            song_info.3 = Self::download_image(coverart_url, song_info_inner.0.clone()).await;
                         }
                        break;
                    }
                }
            }

            
            
            return SongData{
                title: song_info.0,
                artist: song_info.1,
                album: song_info.2,
                duration: duration as f32,
                //image: Some(PathBuf::from(image_path.as_ref().unwrap()))
                image: song_info.3
            };

            });
            tasks.push(task)



        }
        let results = join_all(tasks).await;
        let mut song_data_list: Vec<SongData> = Vec::new();
        for result in results {
            // Unwrapping the tokio::JoinHandle result
            song_data_list.push(result.expect("A task panicked"));
        }

        return song_data_list
    }
    fn get_musicbrainz_id(acoustid_json: &str, duration: u32) -> Option<Vec<String>>{
        let json: Value = serde_json::from_str(acoustid_json).ok()?;

    let mut best_match: Vec<String> = Vec::new(); // (id, title, artist, score)
    for result in json["results"].as_array()? {
        let score = result.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);

        if let Some(recordings) = result.get("recordings") {
            for rec in recordings.as_array()? {
                let rec_id = rec.get("id")?.as_str()?;
                let rec_duration = rec.get("duration").and_then(|d| d.as_f64());
                let rec_title = rec.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown");
                let rec_artist = rec.get("artists")
                    .and_then(|a| a.get(0))
                    .and_then(|ar| ar.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("Unknown");

                // Only consider recordings with duration close to file_duration
                let duration_ok = rec_duration
                    .map(|d| (d - duration as f64).abs() <= 30.0) // ±5 seconds tolerance
                    .unwrap_or(true); // if duration missing, still consider

                if duration_ok {
                    best_match.push(rec_id.to_string());
                    
                }
            }
        }
    }

    Some(best_match)

    }

    async fn download_image(url: Option<String>, name:String) -> Option<PathBuf> {
        let image_file = reqwest::get(url.unwrap()).await.ok()?;
        let image_bytes = image_file.bytes().await.ok()?;
        let kind = infer::get(&image_bytes);
        let ext = kind.map(|k| k.extension()).unwrap_or("png");
        println!("{}",ext);
        let temp_dir = PathBuf::from("./TEMPIMAGES/");
        let name = format!{"cover_image_{}.{}", name,ext};
        let file_path = temp_dir.join(name);
        std::fs::write(&file_path, &image_bytes).ok()?;
        Some(file_path)
    }

    async fn fetch_image(release: Release) -> Option<String> {
        let cover = Release::fetch_coverart()
        .id(&release.id)
        .front()
        .res_1200()
        .execute()
        .await;
        let url =  match cover{
            Ok(CoverartResponse::Url(url)) => {
                return Some(url);
            }
            Ok(CoverartResponse::Json(_coverart_data)) => {
                eprintln!("Unexpected JSON response type when expecting a direct URL.");
            }
            Err(e) => {
                eprintln!("Failed to fetch cover art: {:?}", e);
                // Handle specific errors, e.g., ReleaseNotFound, NetworkError
            }
        };
        None
    }

    async fn fetch_recording_info(id: &str) -> Option<(String,String,Vec<Release>,bool,Vec<Release>,String)> {

        let mut status = false;
        let mut officials = Vec::new();


        let song = Recording::fetch()
        .id(id)
        .with_artists()
        .with_releases()
        .execute().await.expect("Failed to fetch recording");

        let finished = song;

        let artist_id= finished.artist_credit.clone().unwrap().get(0).unwrap().artist.clone().id;
        let artist = finished.artist_credit.clone().unwrap().get(0).unwrap().artist.clone().name;
        let album = finished.releases.unwrap();

        for release in album.clone(){
            if release.status == Some(ReleaseStatus::Official){
                status = true;
                officials.push(release.clone());
            }
        }

        Some((finished.title.clone(),artist,album,status, officials,artist_id))

    }
        
    
    async fn fetch_releases_from_artist(artist: &str) -> Option<(Vec<String>,Vec<String>)> {

        let mut officials = Vec::new();
        let mut others = Vec::new();

        let query = Artist::fetch()
        .id(artist)
        .with_releases()
        .with_release_groups()
        .execute().await.ok()?;

        for i in query.clone().release_groups.unwrap(){
             let is_primary_album = i.primary_type == Some(ReleaseGroupPrimaryType::Album);
            let no_secondary_types = i.secondary_types.is_empty();
            if is_primary_album && no_secondary_types {
                officials.push(i.title.clone());
            } else if is_primary_album && !no_secondary_types{
                others.push(i.title.clone());
            }
        }

        println!("{:?}",officials);
        println!("{:?}",others);
   
        return Some((officials,others));
    }
}
