// Internally, we go from function -> Message -> Handle, so the goal here is to cut out the
// weird middle stuff and do this properly.

use std::time::Duration;
use crate::controller::ButtonLighting;
use crate::types::RGBA;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    SetEnabled(bool),
    KeepAlive,
    SetImage(u32, u32, Vec<u8>),
    SetActiveBrightness(u8),
    SetButtonBrightness(u8),
    SetDimTimeout(Duration),
    SetButtonColour(ButtonLighting, RGBA),
}