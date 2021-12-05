use crate::{
    core::{Button, Key, Result, State},
    load_icon,
};
use async_trait::async_trait;
use image::DynamicImage;
use std::{env, process::Command};

/// A button to screenshot the full desktop using the falmeshot cli.
/// Saves the images in $HOME/Pictures.
pub struct FullScreenshotButton {
    icon: DynamicImage,
}

impl Default for FullScreenshotButton {
    fn default() -> Self {
        let icon = load_icon!("../../icons/screenshot/screenshot.png");

        Self { icon }
    }
}

#[async_trait]
impl Button for FullScreenshotButton {
    async fn init(&mut self, _: &mut State, key: Key) -> Result<()> {
        key.image(self.icon.clone())
    }

    async fn on_click(&mut self, _: &mut State, _: Key) -> Result<()> {
        let home = env::var("HOME").unwrap();
        let path = format!("{}/Pictures", home);
        Command::new("flameshot")
            .args(&["full", "-p", &path])
            .output()
            .unwrap();
        Ok(())
    }
}
