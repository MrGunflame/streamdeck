mod core;
mod log;
mod macros;
mod plugins;

use crate::core::NullButton;
use crate::plugins::{audio, obs, screenshot, vlc};

const VIP: u16 = 0x0fd9;
const PID: u16 = 0x006c;
const SERIAL: &str = "CL17K1A01109";

#[tokio::main]
async fn main() {
    let mut state = core::State::new();

    state.buttons = buttons! {
        audio::DeafenButton,
        audio::MuteButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        screenshot::FullScreenshotButton,
        obs::SaveReplayBufferButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        vlc::PreviousButton,
        vlc::PlayPauseButton,
        vlc::NextButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
        NullButton,
    };

    core::main_loop(VIP, PID, None, state).await;
}

mod pactl {
    use std::error;
    use std::fmt::{self, Display, Formatter};
    use std::io::{BufRead, BufReader};
    use std::process::{ChildStdout, Command, Stdio};
    use std::result;

    #[derive(Debug)]
    pub enum Error {
        DeserializeError,
    }

    type Result<T> = result::Result<T, Error>;

    impl Display for Error {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "{}", "")
        }
    }

    impl error::Error for Error {}

    fn new_pactl() -> Command {
        Command::new("pactl")
    }

    fn string_from_slice(buf: &[u8]) -> String {
        String::from_utf8(buf.into()).unwrap()
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum Event {
        New,
        Change,
        Remove,
    }

    impl Event {
        fn deserialize(buf: &[u8]) -> Option<Self> {
            match buf {
                b"'new'" => Some(Self::New),
                b"'change'" => Some(Self::Change),
                b"'remove'" => Some(Self::Remove),
                _ => None,
            }
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum EventDst {
        Sink(u32),
        Source(u32),
        Card(u32),
        SourceOutput(u32),
        Client(u32),
        SinkInput(u32),
    }

    impl EventDst {
        fn deserialize(buf: [&[u8]; 2]) -> Option<Self> {
            let id = match buf[1].strip_prefix(b"#") {
                Some(id) => std::str::from_utf8(id).unwrap().parse().unwrap(),
                None => return None,
            };

            match buf[0] {
                b"sink" => Some(Self::Sink(id)),
                b"source" => Some(Self::Source(id)),
                b"card" => Some(Self::Card(id)),
                b"source-output" => Some(Self::SourceOutput(id)),
                b"client" => Some(Self::Client(id)),
                b"sink-input" => Some(Self::SinkInput(id)),
                _ => None,
            }
        }
    }

    /// A subscription to `pactl` events using `pactl subscribe`.
    /// Use
    /// # Example
    /// ```
    /// let mut subscription = Subscription::new();
    /// let event = subscription.read_event().expect("Failed to read event");
    /// println!("Event {:?} on {:?}", event.0, event.1);
    /// ```
    // TODO: impl Drop for child spawned by Command.
    pub struct Subscription {
        reader: BufReader<ChildStdout>,
    }

    impl Subscription {
        /// Create a new `Subscription`.
        pub fn new() -> Self {
            let child = Command::new("pactl")
                .arg("subscribe")
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();

            let stdout = child.stdout.unwrap();

            Self {
                reader: BufReader::new(stdout),
            }
        }

        /// Read a single event from the `Subscription`. This method
        /// blocks until a single event was read (or failed).
        pub fn read_event(&mut self) -> Result<(Event, EventDst)> {
            let mut buf = Vec::new();
            self.reader.read_until(b'\n', &mut buf).unwrap();

            // Cut '\n' at the end.
            buf.truncate(buf.len() - 1);

            let parts: Vec<&[u8]> = buf.split(|b| *b == b' ').collect();
            assert_eq!(parts[0], b"Event");
            let event = Event::deserialize(parts[1]).unwrap();
            assert_eq!(parts[2], b"on");
            let dst = match EventDst::deserialize([parts[3], parts[4]]) {
                Some(ev) => ev,
                None => return Err(Error::DeserializeError),
            };

            Ok((event, dst))
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum SinkState {
        Idle,
        Running,
        Suspended,
        Undefined,
    }

    impl Default for SinkState {
        fn default() -> Self {
            Self::Undefined
        }
    }

    impl SinkState {
        fn deserialize(buf: &[u8]) -> Option<Self> {
            match buf {
                b"IDLE" => Some(Self::Idle),
                b"RUNNING" => Some(Self::Running),
                b"SUSPENDED" => Some(Self::Suspended),
                _ => None,
            }
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub struct Sink {
        pub id: u32,
        pub state: SinkState,
        pub name: String,
        pub description: String,
        pub driver: String,
        pub sample_specification: String,
        pub channel_map: String,
        pub owner_module: String,
        pub mute: bool,
        pub volume: String,
        pub base_volume: String,
        pub monitor_source: String,
        pub latency: String,
        pub flags: String,
        pub properties: String,
        pub formats: String,
    }

    impl Sink {
        fn deserialize(buf: &[&[u8]]) -> Option<Self> {
            let mut sink = Self::default();

            for part in buf {
                match part.strip_prefix(b"\t") {
                    Some(part) => {
                        let mut parts: Vec<&[u8]> = part.splitn(2, |b| *b == b':').collect();

                        if parts.len() < 2 {
                            continue;
                        }

                        if parts[1].starts_with(b" ") {
                            parts[1] = &parts[1][1..];
                        }

                        match parts[0] {
                            b"State" => sink.state = SinkState::deserialize(parts[1]).unwrap(),
                            b"Name" => sink.name = string_from_slice(parts[1]),
                            b"Description" => sink.description = string_from_slice(parts[1]),
                            b"Driver" => sink.driver = string_from_slice(parts[1]),
                            b"Sample Specification" => sink.driver = string_from_slice(parts[1]),
                            b"Channel Map" => sink.channel_map = string_from_slice(parts[1]),
                            b"Owner Module" => sink.owner_module = string_from_slice(parts[1]),
                            b"Mute" => {
                                sink.mute = match parts[1] {
                                    b"yes" => true,
                                    _ => false,
                                }
                            }
                            b"Volume" => sink.volume = string_from_slice(parts[1]),
                            b"Monitor Source" => sink.monitor_source = string_from_slice(parts[1]),
                            b"Latency" => sink.latency = string_from_slice(parts[1]),
                            b"Flags" => sink.flags = string_from_slice(parts[1]),
                            b"Properties" => (),
                            b"Formats" => (),
                            _ => (),
                        }
                    }
                    // Start of sink section: "Sink #{id}"
                    None => {
                        let parts: Vec<&[u8]> = part.split(|b| *b == b' ').collect();
                        assert_eq!(parts[0], b"Sink");
                        assert_eq!(parts[1][0], b'#');
                        let id = std::str::from_utf8(&parts[1][1..])
                            .unwrap()
                            .parse()
                            .unwrap();

                        sink.id = id;
                    }
                }
            }

            Some(sink)
        }
    }

    pub fn list_sinks() -> Result<Vec<Sink>> {
        let output = new_pactl().args(&["list", "sinks"]).output().unwrap();

        let output = output.stdout;

        let parts: Vec<&[u8]> = output.split(|b| *b == b'\n').collect();

        let mut sinks_raw = Vec::new();
        {
            // Split sinks
            let mut sink_raw = Vec::new();
            for part in parts {
                match part {
                    b"" => {
                        sinks_raw.push(sink_raw.clone());
                        sink_raw.clear();
                    }
                    _ => sink_raw.push(part),
                }
            }
        }

        let mut sinks = Vec::new();
        for sink_raw in sinks_raw {
            sinks.push(Sink::deserialize(&sink_raw).unwrap());
        }

        Ok(sinks)
    }

    pub enum MuteAction {
        On,
        Off,
        Toggle,
    }

    /// Change the mute state of a sink.
    pub fn set_sink_mute<'life0, T>(sink: T, action: MuteAction) -> Result<()>
    where
        T: Into<&'life0 str>,
    {
        Command::new("pactl")
            .args(&[
                "set-sink-mute",
                sink.into(),
                match action {
                    MuteAction::On => "1",
                    MuteAction::Off => "0",
                    MuteAction::Toggle => "toggle",
                },
            ])
            .output()
            .unwrap();
        Ok(())
    }

    /// Change the mut state of a source.
    pub fn set_source_mute<'life0, T>(sink: T, action: MuteAction) -> Result<()>
    where
        T: Into<&'life0 str>,
    {
        Command::new("pactl")
            .args(&[
                "set-source-mute",
                sink.into(),
                match action {
                    MuteAction::On => "1",
                    MuteAction::Off => "0",
                    MuteAction::Toggle => "toggle",
                },
            ])
            .output()
            .unwrap();
        Ok(())
    }
}

#[macro_export]
macro_rules! buttons {
    ($($button:ty),*$(,)?) => {{
        let mut buttons = ::std::collections::HashMap::new();

        let mut i = 0;
        $(
            buttons.insert(i, $crate::core::ButtonWrapper::new(Box::new(<$button>::default())));

            i += 1;
        )*

        ::std::sync::Arc::new(::std::sync::RwLock::new(buttons))
    }};
}
