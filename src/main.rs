use std::sync::Arc;
mod webapi;
mod error;
mod data;
use data::SearchTopic;
use crate::data::{TrackId, Track};
use psst_core::{
    audio::{normalize::NormalizationLevel, output::AudioOutput},
    cache::{Cache, CacheHandle},
    cdn::{Cdn, CdnHandle},
    connection::Credentials,
    error::Error,
    item_id::{ItemId, ItemIdType},
    player::{item::PlaybackItem, PlaybackConfig, Player, PlayerCommand, PlayerEvent},
    session::{SessionConfig, SessionService},
};
use std::{env, io, io::BufRead, path::PathBuf, thread};
use druid::{im::Vector};
use webapi::WebApi;


fn main() {
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
    let result = api.search("hello nasty", &[SearchTopic::Album]).unwrap();
    let best_album = result.albums[0].clone();
    println!("Best album: {}", best_album.name);
    start_album(best_album.id.clone(), session).unwrap();
}

fn start_album(album_id: Arc<str>, session: SessionService) -> Result<(), Error> {
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
