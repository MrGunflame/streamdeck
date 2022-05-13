use crate::pactl::{
    list_sinks, set_sink_mute, set_source_mute, Event, EventDst, MuteAction, Subscription,
};
use crate::{
    core::{Button, Error, Key, Result, State},
    load_icon,
};
use async_trait::async_trait;
use image::DynamicImage;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DEFAULT_SINK: &str = "alsa_output.pci-0000_0a_00.4.analog-stereo";
const DEFAULT_SOURCE: &str = "";

/// Deafen/Undeafen the system-wide audio output stream.
#[derive(Clone, Debug)]
pub struct DeafenButton {
    mute: Arc<AtomicBool>,
    icons: [DynamicImage; 2],
}

impl DeafenButton {
    /// Rerender the button based on the value `value`.
    fn render(&self, value: bool, key: Key) -> Result<()> {
        match value {
            false => key.image(self.icons[0].clone()),
            true => key.image(self.icons[1].clone()),
        }
    }

    /// Invert the value of the `mute` field and rerender the button.
    /// This does not change the audio stream itself.
    fn toggle(&self, key: Key) -> Result<()> {
        let value = self.mute.load(Ordering::SeqCst);
        self.mute.store(!value, Ordering::SeqCst);
        self.render(!value, key)
    }
}

impl Default for DeafenButton {
    fn default() -> Self {
        let icon_mute_off = load_icon!("../../icons/audio/audio_deaf_off.png");
        let icon_mute_on = load_icon!("../../icons/audio/audio_deaf_on.png");

        Self {
            mute: Arc::new(AtomicBool::new(false)),
            icons: [icon_mute_off, icon_mute_on],
        }
    }
}

#[async_trait]
impl Button for DeafenButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        // Create a new `Arc` pointing to `self` to allow the task listening
        // on pactl events to mutate data.
        let self_ref = Arc::new(self.clone());

        // The id of the default sink.
        let default_sink = DEFAULT_SINK;

        {
            let key = key.clone();
            std::thread::spawn(move || {
                // Create a new pactl event subscription and read all events. Only proceed
                // when the event changes a property on the default sink.
                let mut pactl_subscription = Subscription::new();
                loop {
                    while let Ok(event) = pactl_subscription.read_event() {
                        // Ony listen on sink changes.
                        if event.0 == Event::Change
                            && match event.1 {
                                EventDst::Sink(_) => true,
                                _ => false,
                            }
                        {
                            // Get all sinks.
                            let sinks = list_sinks().unwrap();
                            // Find the default sink.
                            let sink = match sinks.iter().find(|s| s.name == default_sink) {
                                Some(sink) => sink,
                                None => continue,
                            };
                            // If the data from the actual sink missmatches the current state
                            // swap the bool and rerender the key.
                            if sink.mute != self_ref.mute.load(Ordering::SeqCst) {
                                self_ref.toggle(key.clone()).unwrap();
                            }
                        }
                    }
                }
            });
        }

        self.render(false, key)
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        match set_sink_mute("@DEFAULT_SINK@", MuteAction::Toggle) {
            Ok(()) => Ok(()),
            Err(err) => Err(Error::BoxError(Box::new(err))),
        }
    }
}

#[derive(Debug)]
pub struct MuteButton {
    mute: bool,
    icons: [DynamicImage; 2],
}

impl MuteButton {
    fn render(&self, key: Key) -> Result<()> {
        match self.mute {
            false => key.image(self.icons[0].clone()),
            true => key.image(self.icons[1].clone()),
        }
    }
}

impl Default for MuteButton {
    fn default() -> Self {
        let icon_mute_off = load_icon!("../../icons/audio/audio_mute_off.png");
        let icon_mute_on = load_icon!("../../icons/audio/audio_mute_on.png");

        Self {
            mute: false,
            icons: [icon_mute_off, icon_mute_on],
        }
    }
}

#[async_trait]
impl Button for MuteButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        self.render(key)
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        match set_source_mute("@DEFAULT_SOURCE@", MuteAction::Toggle) {
            Ok(()) => Ok(()),
            Err(err) => Err(Error::BoxError(Box::new(err))),
        }
    }
}
