use reqwest;
use crate::song::Song;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrcLibResponse {
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
    track_name: String,
    artist_name: String,
    album_name: Option<String>,
    duration: f64,
    instrumental: bool,
}

pub async fn get_lyrics(song: Song) -> String{
    let resp = reqwest::Client::builder().
    user_agent("clean-music/0.1")
    .build()
    .unwrap()
    .get("https://lrclib.net/api/get")
    .query(&[
        ("track_name", song.title.as_str()),
        ("artist_name", song.artist.as_str()),
        ("album_name", song.album.as_str()),
        ("duration", &song.duration.as_secs().to_string()),
    ])
    .send()
    .await
    .ok();

    match resp {
      Some(res) =>   {
        match res.json::<LrcLibResponse>().await{
          Ok(jtext) => {
            match jtext.plain_lyrics{
              Some(text) => return text,
              _ => return "LYRICS NOT FOUND".to_string()
            }
          },
          _ => return "SONG NOT FOUND".to_string() 
        }
      },
      _ => return "COULD NOT REACH WEB".to_string()

    }
}