use crate::{
    core::{Button, Error, Key, Result, State},
    debug, load_icon,
};
use image::DynamicImage;
use obws::Client;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
    time::{self, Duration},
};

const OBS_CLIENT_HOST: &str = "127.0.0.1";
const OBS_CLIENT_PORT: u16 = 4444;

/// Try to reconnect even n seconds if the connection failed.
const OBS_CLIENT_RECONNECT: Option<Duration> = Some(Duration::from_secs(60));

/// OBS WebSocket client shared between all buttons. Used to communicate
/// with OBS using just a single connection.
#[derive(Clone)]
struct OBSClient {
    tx: mpsc::Sender<Message>,
}

enum Message {
    SaveReplayBuffer,
}

impl OBSClient {
    async fn new(state: &mut State) -> std::result::Result<(), obws::Error> {
        if !state.exists::<Self>() {
            let (tx, mut rx) = mpsc::channel(32);
            task::spawn(async move {
                // let client = Client::connect(OBS_CLIENT_HOST, OBS_CLIENT_PORT).await;

                loop {
                    let client = match Client::connect(OBS_CLIENT_HOST, OBS_CLIENT_PORT).await {
                        Ok(client) => client,
                        Err(err) => {
                            eprintln!("[OBS] Failed to connect: {:?}", err);
                            time::sleep(match OBS_CLIENT_RECONNECT {
                                Some(dur) => dur,
                                None => break,
                            })
                            .await;
                            continue;
                        }
                    };

                    while let Some(msg) = rx.recv().await {
                        match msg {
                            Message::SaveReplayBuffer => {
                                let res = client.replay_buffer().save_replay_buffer().await;
                                // let _ = tx.send(res);
                                println!("{:?}", res);
                            }
                        }
                    }
                }
            });

            state.insert(Self { tx });
        }

        Ok(())
    }

    pub async fn send(&self, msg: Message) -> Result<()> {
        let _ = self.tx.send(msg).await;
        Ok(())
    }
}

/// Save and flush the current replay buffer it it exists.
pub struct SaveReplayBufferButton {
    icon: DynamicImage,
}

impl Default for SaveReplayBufferButton {
    fn default() -> Self {
        let icon = load_icon!("../../icons/obs/obs.png");

        Self { icon }
    }
}

#[async_trait::async_trait]
impl Button for SaveReplayBufferButton {
    async fn init(&mut self, state: &mut State, key: Key) -> Result<()> {
        OBSClient::new(state).await.unwrap();
        key.image(self.icon.clone())
    }

    async fn on_click(&mut self, state: &mut State, key: Key) -> Result<()> {
        let client = {
            let client = state.shared_data.read().unwrap();
            let client = client.get::<OBSClient>().unwrap();
            client.clone()
        };

        let _ = client.send(Message::SaveReplayBuffer).await;

        Ok(())
    }
}
