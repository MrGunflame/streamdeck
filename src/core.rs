use crate::{debug, info};
use image::DynamicImage;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::convert::{From, Into};
use std::error;
use std::process;
use std::result;
use std::sync::{mpsc, Arc, RwLock, RwLockReadGuard};
use std::thread;
use std::time::Duration;

const POLLING_RATE: Duration = Duration::from_millis(500);

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Streamdeck(streamdeck::Error),
    BoxError(Box<dyn error::Error>),
}

impl From<streamdeck::Error> for Error {
    fn from(err: streamdeck::Error) -> Self {
        Self::Streamdeck(err)
    }
}

pub struct TypeMap(HashMap<TypeId, Box<(dyn Any + Send + Sync)>>);

impl TypeMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.0.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn contains_key<T>(&self) -> bool
    where
        T: 'static,
    {
        self.0.contains_key(&TypeId::of::<T>())
    }
}

#[derive(Clone)]
pub struct State {
    pub buttons: Arc<RwLock<HashMap<u8, ButtonWrapper>>>,
    pub shared_data: Arc<RwLock<TypeMap>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            buttons: Arc::new(RwLock::new(HashMap::new())),
            shared_data: Arc::new(RwLock::new(TypeMap::new())),
        }
    }

    /// WIP
    // pub fn get<T>(&mut self) -> RwLockReadGuard<&T>
    // where
    //     T: 'static,
    // {
    //     let shared_data = self.shared_data.read().unwrap();

    //     shared_data.get(&TypeId::of::<T>())
    // }

    pub fn type_of<T>() -> TypeId
    where
        T: 'static,
    {
        TypeId::of::<T>()
    }

    pub fn exists<T>(&self) -> bool
    where
        T: 'static,
    {
        let shared_data = self.shared_data.read().unwrap();

        shared_data.contains_key::<T>()
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync + 'static,
    {
        let mut shared_data = self.shared_data.write().unwrap();

        shared_data.insert(value);
    }
}

pub struct LOCK<'life0, T> {
    lock: &'life0 RwLock<T>,
}

pub struct ButtonWrapper {
    button: Box<dyn Button>,
}

impl ButtonWrapper {
    pub fn new(button: Box<dyn Button>) -> Self {
        Self { button }
    }

    /// Call the `init` method of the button.
    async fn exec_init(
        &mut self,
        key: u8,
        streamdeck: StreamDeck,
        state: &mut State,
    ) -> Result<()> {
        self.button.init(state, Key::new(key, streamdeck)).await
    }

    /// Call the `on_click` method of the button.
    async fn exec_click(
        &mut self,
        key: u8,
        streamdeck: StreamDeck,
        state: &mut State,
    ) -> Result<()> {
        self.button.on_click(state, Key::new(key, streamdeck)).await
    }
}

pub async fn main_loop(vid: u16, pid: u16, serial: Option<String>, mut state: State) -> ! {
    let deck = match StreamDeck::connect(vid, pid, serial) {
        Ok(deck) => deck,
        Err(err) => {
            println!("[FATAL] Failed to connect to Streamdeck: {:?}", err);
            process::exit(1);
        }
    };

    info!("Connected to streamdeck (VID = {}, PID = {})", vid, pid);

    // Call the `init` method on every button.

    let buttons = state.buttons.clone();
    for (key, button) in buttons.write().unwrap().iter_mut() {
        match button.exec_init(*key, deck.clone(), &mut state).await {
            Ok(()) => (),
            Err(err) => println!("[ERROR] Failed to initialize key {}: {:?}", key, err),
        }
    }

    loop {
        // Wait for a button to be pressed (or released).
        let (tx, rx) = mpsc::channel();

        deck.send(Message::ReadButtons(tx)).unwrap();
        let keys = match rx.recv().unwrap() {
            Some(keys) => keys,
            None => {
                thread::sleep(POLLING_RATE);
                continue;
            }
        };

        // Find the pressed button.
        let key = match keys.iter().enumerate().find(|&(_, &x)| x == 1) {
            Some((i, _)) => i as u8,
            None => continue,
        };

        #[cfg(debug_assertions)]
        debug!("Key {} (ROW {} COL {}) pressed", key, key / 8, key % 8);

        // Execute the buttons job.
        {
            let buttons = state.buttons.clone();
            let mut buttons = buttons.write().unwrap();
            match buttons.get_mut(&key) {
                Some(button) => match button.exec_click(key, deck.clone(), &mut state).await {
                    Ok(()) => (),
                    Err(err) => println!("[ERROR] Error executing job for key {}: {:?}", key, err),
                },
                None => (),
            }
        }
    }
}

enum Message {
    SetColor(u8, Color),
    SetImage(u8, DynamicImage),
    ReadButtons(mpsc::Sender<Option<Vec<u8>>>),
}

#[derive(Clone, Debug)]
pub struct StreamDeck {
    tx: mpsc::Sender<Message>,
}

impl StreamDeck {
    pub fn connect(vid: u16, pid: u16, serial: Option<String>) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let mut deck = streamdeck::StreamDeck::connect(vid, pid, serial)?;

        deck.set_blocking(false)?;

        std::thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    Message::SetColor(key, color) => {
                        deck.set_button_rgb(key, &color.into()).unwrap()
                    }
                    Message::SetImage(key, image) => deck.set_button_image(key, image).unwrap(),
                    Message::ReadButtons(tx) => {
                        let keys = match deck.read_buttons(None) {
                            Ok(keys) => Some(keys),
                            Err(err) => match err {
                                streamdeck::Error::NoData => None,
                                _ => panic!("{:?}", err),
                            },
                        };

                        let _ = tx.send(keys);
                    }
                }
            }
        });

        Ok(Self { tx })
    }

    fn send(&self, msg: Message) -> Result<()> {
        let _ = self.tx.send(msg);
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait Button: Send + Sync {
    async fn init(&mut self, state: &mut State, key: Key) -> Result<()>;
    async fn on_click(&mut self, state: &mut State, key: Key) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct Key {
    key: u8,
    deck: StreamDeck,
}

impl Key {
    fn new(key: u8, deck: StreamDeck) -> Self {
        Self { key, deck }
    }

    /// Set the key to a constant color.
    pub fn color<T>(&self, color: T) -> Result<()>
    where
        T: Into<Color>,
    {
        self.deck.send(Message::SetColor(self.key, color.into()))
    }

    pub fn image(&self, image: DynamicImage) -> Result<()> {
        self.deck.send(Message::SetImage(self.key, image))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<(u8, u8, u8)> for Color {
    fn from(t: (u8, u8, u8)) -> Self {
        Self {
            r: t.0,
            g: t.1,
            b: t.2,
        }
    }
}

impl From<u32> for Color {
    fn from(t: u32) -> Self {
        Self {
            r: t as u8,
            g: (t << 8) as u8,
            b: (t << 16) as u8,
        }
    }
}

impl From<Color> for streamdeck::Colour {
    fn from(t: Color) -> Self {
        Self {
            r: t.r,
            g: t.g,
            b: t.b,
        }
    }
}

/// A button that does nothing.
pub struct NullButton;

impl Default for NullButton {
    fn default() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Button for NullButton {
    async fn init(&mut self, _: &mut State, _: Key) -> Result<()> {
        Ok(())
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        Ok(())
    }
}

#[cfg(tests)]
mod tests {
    use super::Color;

    #[test]
    fn test_color() {
        assert_eq!(
            Color::from((32, 65, 128)),
            Color {
                r: 32,
                g: 56,
                b: 128
            }
        );
    }
}
