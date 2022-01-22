use std::sync::Arc;
use std::collections::HashMap;
use data::SearchTopic;
use crate::data::TrackId;
use psst_core::{
    audio::{normalize::NormalizationLevel, output::AudioOutput},
    cache::{Cache, CacheHandle},
    cdn::{Cdn, CdnHandle},
    connection::Credentials,
    error::Error,
    player::{item::PlaybackItem, PlaybackConfig, Player, PlayerCommand, PlayerEvent},
    session::{SessionConfig, SessionService},
};
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_channel::mpsc::UnboundedSender;
use std::{env, io, io::BufRead, path::PathBuf, thread};
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use rustcroft::MycroftMessage;

mod bus_connection;
mod error;
mod data;
mod webapi;

use data::Album;
use webapi::WebApi;
use bus_connection::{start_spotify_service, MsgHandler};





fn search_album(query: &str, api: &WebApi) -> Result<Arc<Album>, error::Error> {
    let result = api.search(query, &[SearchTopic::Album])?;
    let best_album = result.albums[0].clone();
    println!("Best album: {}", best_album.name);
    Ok(best_album)
}


#[derive(Serialize, Deserialize)]
pub struct SearchData {
    pub id: String,
    pub name: String,
    pub media_type: String
}

fn search_handler(message: serde_json::Value,
                  bus_tx: &UnboundedSender<Message>,
                  api: &WebApi,
                  _session: &SessionService) {
    println!("Searching Spotify");
    let query = message["data"]["query"].to_string();
    println!("Query: {}", query);

    let best_album = search_album(query.as_str(), &api).unwrap();
    let search_result = SearchData{id: best_album.id.to_string(),
                                   name: best_album.name.to_string(),
                                   media_type: "album".to_string()};
    println!("Found {} ({})", search_result.name, search_result.id);
    let response = MycroftMessage::new("spotify.search.response")
        .with_data(serde_json::to_value(search_result).unwrap());
    bus_tx.unbounded_send(response.to_message()).unwrap();
}

fn play_handler(message: serde_json::Value,
                _bus: &UnboundedSender<Message>,
                _api: &WebApi,
                session: &SessionService) {
        let query = message["data"]["album"].as_str().unwrap();
        start_album(Arc::new(query), session.clone()).unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();

    //let args: Vec<String> = env::args().collect();
    let login_creds = Credentials::from_username_and_password(
        env::var("SPOTIFY_USERNAME").unwrap(),
        env::var("SPOTIFY_PASSWORD").unwrap(),
    );
    let session = SessionService::with_config(SessionConfig {
        login_creds,
        proxy_url: None,
    });
    let api = WebApi::new(session.clone(),
                          None,
                          Some(PathBuf::from("/tmp")));


    let mut handlers = HashMap::<String, MsgHandler>::new();
    handlers.insert("spotify.search".to_string(), search_handler);
    handlers.insert("spotify.play".to_string(), play_handler);

	start_spotify_service(handlers, api, session).await;
}

fn start_album(album_id: Arc<&str>, session: SessionService) -> Result<(), Error> {
    let cdn = Cdn::new(session.clone(), None)?;
    let cache = Cache::new(PathBuf::from("cache"))?;
    let api = WebApi::new(session.clone(),
                          None,
                          Some(PathBuf::from("/tmp")));
    let album = api.get_album(&album_id).unwrap();

    let mut playlist = Vec::<PlaybackItem>::new();
    for track in &album.data.tracks {
        let item = PlaybackItem {
            item_id: *track.id,
            norm_level: NormalizationLevel::Track,
        };
        playlist.push(item);
    }
    play_items(
        session,
        cdn,
        cache,
        playlist 
    )
}


fn start_track(track_id: TrackId, session: SessionService) -> Result<(), Error> {
    let cdn = Cdn::new(session.clone(), None)?;
    let cache = Cache::new(PathBuf::from("cache"))?;
    play_items(
        session,
        cdn,
        cache,
        vec![PlaybackItem {
             //item_id,
             item_id: *track_id,
             norm_level: NormalizationLevel::Track,
         }],
    )
}

fn play_items(
    session: SessionService,
    cdn: CdnHandle,
    cache: CacheHandle,
    items: Vec<PlaybackItem>,
) -> Result<(), Error> {
    let output = AudioOutput::open()?;
    let config = PlaybackConfig::default();

    let mut player = Player::new(session, cdn, cache, config, &output);

    let _ui_thread = thread::spawn({
        let player_sender = player.sender();

        player_sender
            .send(PlayerEvent::Command(PlayerCommand::LoadQueue {
                items: items,
                position: 0,
            }))
            .unwrap();

        move || {
            for line in io::stdin().lock().lines() {
                match line.as_ref().map(|s| s.as_str()) {
                    Ok("p") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Pause))
                            .unwrap();
                    }
                    Ok("r") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Resume))
                            .unwrap();
                    }
                    Ok("s") => {
                        println!("STOP COMMAND!");
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Stop))
                            .unwrap();
                    }
                    Ok("<") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Previous))
                            .unwrap();
                    }
                    Ok(">") => {
                        player_sender
                            .send(PlayerEvent::Command(PlayerCommand::Next))
                            .unwrap();
                    }
                    _ => log::warn!("unknown command"),
                }
            }
        }
    });

    for event in player.receiver() {
        player.handle(event);
    }
    output.sink().close();

    Ok(())
}
