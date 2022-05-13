use crate::{
    core::{Button, Key, Result, State},
    load_icon,
};
use async_trait::async_trait;
use image::DynamicImage;
use std::process::Command;

#[derive(Debug)]
pub struct PlayPauseButton {
    pause: bool,
    icons: [DynamicImage; 2],
}

impl PlayPauseButton {
    fn render(&self, key: Key) -> Result<()> {
        match self.pause {
            false => key.image(self.icons[0].clone()),
            true => key.image(self.icons[1].clone()),
        }
    }
}

impl Default for PlayPauseButton {
    fn default() -> Self {
        let icon_play = load_icon!("../../icons/vlc/vlc_playpause_play.png");
        let icon_pause = load_icon!("../../icons/vlc/vlc_playpause_pause.png");

        Self {
            pause: false,
            icons: [icon_play, icon_pause],
        }
    }
}

#[async_trait]
impl Button for PlayPauseButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        self.render(key)
    }

    async fn on_click(&mut self, _: &mut State, key: Key) -> Result<()> {
        let output = vlc_dbus_send("org.mpris.MediaPlayer2.Player.PlayPause");

        #[cfg(debug_assertions)]
        println!("[VLC] [PlayPause] {:?}", output);

        self.pause = !self.pause;
        self.render(key)
    }
}

#[derive(Debug)]
pub struct NextButton {
    icon: DynamicImage,
}

impl Default for NextButton {
    fn default() -> Self {
        let icon = load_icon!("../../icons/vlc/vlc_next.png");

        Self { icon }
    }
}

#[async_trait]
impl Button for NextButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        key.image(self.icon.clone())
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        let output = vlc_dbus_send("org.mpris.MediaPlayer2.Player.Next");

        #[cfg(debug_assertions)]
        println!("[VLC] [Next] {:?}", output);

        Ok(())
    }
}

#[derive(Debug)]
pub struct PreviousButton {
    icon: DynamicImage,
}

impl Default for PreviousButton {
    fn default() -> Self {
        let icon = load_icon!("../../icons/vlc/vlc_previous.png");

        Self { icon }
    }
}

#[async_trait]
impl Button for PreviousButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        key.image(self.icon.clone())
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        let output = vlc_dbus_send("org.mpris.MediaPlayer2.Player.Previous");

        #[cfg(debug_assertions)]
        println!("[VLC] [Previous] {:?}", output);

        Ok(())
    }
}

fn vlc_dbus_send(message: &str) -> std::process::Output {
    Command::new("dbus-send")
        .args(&[
            "--print-reply",
            "--session",
            "--dest=org.mpris.MediaPlayer2.vlc",
            "/org/mpris/MediaPlayer2",
            message,
        ])
        .output()
        .unwrap()
}
