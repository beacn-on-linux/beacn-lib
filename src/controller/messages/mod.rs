// Internally, we go from function -> Message -> Handle, so the goal here is to cut out the
// weird middle stuff and do this properly.

use crate::controller::ButtonLighting;
use crate::types::RGBA;
use std::sync::Arc;
use web_time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    KeepAlive,
    Enabled(bool),
    Image(u32, u32, Arc<Vec<u8>>),
    DisplayBrightness(u8),
    ButtonBrightness(u8),
    DimTimeout(Duration),
    ButtonColour(ButtonLighting, RGBA),
}
