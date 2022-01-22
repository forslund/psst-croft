use tokio_tungstenite::{connect_async};
use url::Url;

use std::collections::HashMap;
use futures_channel::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;
use serde_json::{Value};
use futures_util::{future, pin_mut, StreamExt};

use psst_core::session::SessionService;

use crate::webapi::WebApi;

pub type MsgHandler = fn(serde_json::Value, &UnboundedSender<Message>,
                         &WebApi, &SessionService);

pub struct EventHandler {
    handlers: HashMap<String, MsgHandler>,
    api: WebApi,
    session: SessionService
}


impl EventHandler {
    #[allow(dead_code)]
    pub fn add(&mut self, name: &str, func: MsgHandler) {
        let key = format!("\"{}\"", name);
        self.handlers.insert(key, func);
    }

    #[allow(dead_code)]
    pub fn call(&self, name: &str, msg: serde_json::Value, bus_tx: &UnboundedSender<Message>) {
        for (key, _) in &self.handlers {
            println!("{}", key);
        }
        match self.handlers.get(name) {
            Some(handler) => handler(msg, &bus_tx, &self.api, &self.session),
            None => println!("No handler found for messagetype {}", name)
        }
    }

    pub async fn handle_msg(&self, bus_tx: &UnboundedSender<Message>,
                        data: Vec<u8>) -> serde_json::Result<()> {
        let s = std::str::from_utf8(&data).unwrap();
        let msg: Value = serde_json::from_str(&s)?;
        //TODO: Better Error handling
        let msg_type = &msg["type"].as_str().unwrap();
        println!("Message: {}", msg_type);
        self.call(msg_type, msg.clone(), &bus_tx);
        Ok(())
    }

    #[allow(dead_code)]
    /// Create a new event handler
    pub fn new(handlers: HashMap<String, MsgHandler>,
               api: WebApi, session: SessionService) -> EventHandler {
        EventHandler{ handlers: handlers, api: api, session: session }
    }
}

/// Register intents and connect the skill to the message bus
pub async fn start_spotify_service(handlers: HashMap<String, MsgHandler>,
                                   api: WebApi,
                                   session: SessionService) {
    let (bus_tx, bus_rx) = futures_channel::mpsc::unbounded();
	let bus_handler = EventHandler::new(handlers, api, session);

    let url = Url::parse("ws://localhost:8181/core").unwrap();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();
    let write_to_ws = bus_rx.map(Ok).forward(write);

    let handle_message = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            bus_handler.handle_msg(&bus_tx, data).await.unwrap();
        })
    };

    pin_mut!(write_to_ws, handle_message);
    future::select(write_to_ws, handle_message).await;
}
