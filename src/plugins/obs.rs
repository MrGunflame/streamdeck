use crate::{
    core::{Button, Error, Key, Result, State},
    load_icon,
};
use async_trait::async_trait;
use image::DynamicImage;
use obws::{responses::RecordingStatus, Client};
use tokio::{
    sync::{mpsc, oneshot},
    task,
    time::{self, Duration},
};

const OBS_CLIENT_HOST: &str = "127.0.0.1";
const OBS_CLIENT_PORT: u16 = 4444;

/// Try to reconnect even n seconds if the connection failed.
const OBS_CLIENT_RECONNECT: Option<Duration> = None;

/// OBS WebSocket client shared between all buttons. Used to communicate
/// with OBS using just a single connection.
#[derive(Clone, Debug)]
struct OBSClient {
    tx: mpsc::Sender<Message>,
}

enum Message {
    RecordingStatus(oneshot::Sender<Result<RecordingStatus>>),
    RecordingStart(oneshot::Sender<Result<()>>),
    RecordingStop(oneshot::Sender<Result<()>>),
    SaveReplayBuffer,
}

impl OBSClient {
    async fn new(state: &mut State) -> std::result::Result<(), obws::Error> {
        // Skip adding a new `OBSClient` when one already exists
        // in the typemap.
        let typemap = state.typemap.read().unwrap();
        if typemap.contains_key::<Self>() {
            return Ok(());
        }
        drop(typemap);

        let mut typemap = state.typemap.write().unwrap();
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
                        Message::RecordingStatus(tx) => {
                            let res = client.recording().get_recording_status().await;

                            let res = res.or_else(|e| Err(e.into()));
                            let _ = tx.send(res);
                        }
                        Message::RecordingStart(tx) => {
                            let res = client.recording().start_recording().await;

                            let res = res.or_else(|e| Err(e.into()));
                            let _ = tx.send(res);
                        }
                        Message::RecordingStop(tx) => {
                            let res = client.recording().stop_recording().await;

                            let res = res.or_else(|e| Err(e.into()));
                            let _ = tx.send(res);
                        }

                        Message::SaveReplayBuffer => {
                            let res = client.replay_buffer().save_replay_buffer().await;
                            // let _ = tx.send(res);
                            println!("{:?}", res);
                        }
                    }
                }
            }
        });

        typemap.insert(Self { tx });
        Ok(())
    }

    async fn send(&self, msg: Message) -> Result<()> {
        let _ = self.tx.send(msg).await;
        Ok(())
    }

    /// Returns the current recording status of the OBS
    /// client.
    async fn recording_status(&self) -> Result<RecordingStatus> {
        let (tx, rx) = oneshot::channel();
        let _ = self.send(Message::RecordingStatus(tx)).await;

        match rx.await {
            Ok(res) => res,
            Err(_) => Err(Error::NoResponse),
        }
    }

    /// Starts recording on the OBS client. Returns an error
    /// when the client is already recording.
    async fn recording_start(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self.send(Message::RecordingStart(tx)).await;

        match rx.await {
            Ok(res) => res,
            Err(_) => Err(Error::NoResponse),
        }
    }

    /// Stops recording on the OBS client. Returns an error
    /// when the client is not recording.
    async fn recording_stop(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self.send(Message::RecordingStop(tx)).await;

        match rx.await {
            Ok(res) => res,
            Err(_) => Err(Error::NoResponse),
        }
    }
}

/// A button to toggle the current recording status
/// in OBS.
#[derive(Debug)]
pub struct RecordingButton {}

#[async_trait]
impl Button for RecordingButton {
    async fn init(&mut self, state: &mut State, key: Key) -> Result<()> {
        OBSClient::new(state).await.unwrap();

        key.color((0, 0, 250))
    }

    async fn on_click(&mut self, state: &mut State, _key: Key) -> Result<()> {
        let client = get_client_from_state(state);

        let status = client.recording_status().await?;

        match status.is_recording {
            // Stop the recording.
            true => client.recording_stop().await?,
            // Start the recording.
            false => client.recording_start().await?,
        }

        Ok(())
    }
}

/// Save and flush the current replay buffer it it exists.
#[derive(Debug)]
pub struct SaveReplayBufferButton {
    icon: DynamicImage,
}

impl Default for SaveReplayBufferButton {
    fn default() -> Self {
        let icon = load_icon!("../../icons/obs/obs.png");

        Self { icon }
    }
}

#[async_trait]
impl Button for SaveReplayBufferButton {
    async fn init(&mut self, state: &mut State, key: Key) -> Result<()> {
        OBSClient::new(state).await.unwrap();

        key.image(self.icon.clone())
    }

    async fn on_click(&mut self, state: &mut State, _key: Key) -> Result<()> {
        let client = get_client_from_state(state);

        let _ = client.send(Message::SaveReplayBuffer).await;
        Ok(())
    }
}

/// Returns a cloned [`OBSClient`] from the global [`State`].
fn get_client_from_state(state: &State) -> OBSClient {
    let typemap = state.typemap.read().unwrap();
    let client = typemap.get::<OBSClient>().unwrap();
    client.clone()
}
