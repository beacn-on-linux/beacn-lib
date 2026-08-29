//! These are special messages where multiple values are fetched at once, which is different from
//! the more traditional Key -> Value setting. These are primarily polled to display stuff in the UI

use anyhow::{Result, bail};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug, Copy, Clone)]
pub enum BulkMessage {
    GetMeters,
    Meters(MetersResponse),

    GetSuppressionBase,
    SuppressionBase(SuppressionResponse),

    GetSuppressionCurrent,
    SuppressionCurrent(SuppressionResponse),
}

impl BulkMessage {
    pub fn is_valid_fetch(&self) -> bool {
        matches!(
            self,
            BulkMessage::GetMeters
                | BulkMessage::GetSuppressionBase
                | BulkMessage::GetSuppressionCurrent
        )
    }

    pub fn to_beacn_key(&self) -> [u8; 3] {
        match self {
            BulkMessage::GetMeters => [0x00, 0x00, 0x00],
            BulkMessage::GetSuppressionBase => [0x04, 0x00, 0x00],
            BulkMessage::GetSuppressionCurrent => [0x06, 0x00, 0x00],
            _ => panic!("Attempted to Get a Response"),
        }
    }

    // We only actually care about the first 68 bytes of the response, the first four bytes are
    // the header which can be ignored, and the rest constitutes 16 (maybe) LE floats which should
    // be filled into the response struct and sent back across the channel
    pub fn handle_response(&self, response: &[u8]) -> Result<BulkMessage> {
        if response.len() < 68 {
            bail!(
                "Invalid response length: expected at least 68 bytes, got {}",
                response.len()
            );
        }

        // Ignore the first four bytes, which are the response header.
        let mut cursor = Cursor::new(&response[4..68]);

        let msg = match self {
            BulkMessage::GetMeters => {
                let response = MetersResponse {
                    raw_amplitude: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    limiter_output: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    limiter_adjustment: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    bass_response: cursor.read_f32::<LittleEndian>()?,
                    hp_subwoofer_meter: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    treble_response: cursor.read_f32::<LittleEndian>()?,
                    compressor_attenuation: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    processed_mic: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    pre_expander: cursor.read_f32::<LittleEndian>()?,
                    post_expander: cursor.read_f32::<LittleEndian>()?,
                    pre_compressor: cursor.read_f32::<LittleEndian>()?,
                    post_compressor: cursor.read_f32::<LittleEndian>()?,
                    float_12: cursor.read_f32::<LittleEndian>()?,
                    hp_meter_left: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    hp_meter_right: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    clip_warning: cursor.read_f32::<LittleEndian>()?,
                };

                BulkMessage::Meters(response)
            }

            BulkMessage::GetSuppressionBase | BulkMessage::GetSuppressionCurrent => {
                // 16 floats here too,
                let response = SuppressionResponse {
                    float_0: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_1: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_2: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_3: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_4: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_5: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_6: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_7: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_8: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_9: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_10: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_11: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_12: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_13: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_14: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                    float_15: lin_to_db(cursor.read_f32::<LittleEndian>()?),
                };

                match self {
                    BulkMessage::GetSuppressionBase => BulkMessage::SuppressionBase(response),
                    BulkMessage::GetSuppressionCurrent => BulkMessage::SuppressionCurrent(response),
                    _ => unreachable!(),
                }
            }

            _ => unreachable!(),
        };

        Ok(msg)
    }
}

// Ok, all values appear to be in dBFS
#[derive(Debug, Copy, Clone)]
pub struct MetersResponse {
    /// Raw internal mic amplitude, this looks like dBFS but can go above 0 (activating the clip
    /// warning). This is basically the value pre-limiter.
    ///
    /// Value Type: dBFS
    pub raw_amplitude: f32,

    /// Raw mic amplitude after the limiter is applied, this SHOULD be below 0dBFS.
    ///
    /// Value Type: dBFS
    pub limiter_output: f32,

    /// Adjustment made by the limiter. This reacts immediately to a clip but also appears
    /// to decay over time.
    ///
    /// Value Type: dBFS
    pub limiter_adjustment: f32,

    /// Responds to noise at roughly around 50-100hz, likely used in EQ drawing.
    ///
    /// Value Type: Unknown
    pub bass_response: f32,

    /// The applied headphones subwoofer amount.
    ///
    /// Value Type: dBFS
    pub hp_subwoofer_meter: f32,

    /// Responds to noise at roughly around 5khz, likely used in EQ drawing.
    ///
    /// Value Type: Unknown
    pub treble_response: f32,

    /// The amount of attenuation applied by the compressor. This pulls from the top, so a value
    /// of 0dBFS means it's not doing anything. Used to draw Red bar in the official app.
    ///
    /// Value Type: dBFS
    pub compressor_attenuation: f32,

    /// The final processed mic amplitude after the entire filter change has been applied. This
    /// is used in the far right mic meter in the official app.
    ///
    /// Value Type: dBFS
    pub processed_mic: f32,

    /// The current full microphone amplitude (behaviour if expander was disabled).
    ///
    /// Value Type: dBFS
    pub pre_expander: f32,

    /// The microphone amplitude after the expander has been applied. Depending on things like the
    ///  ratio, this might 'activate' below the threshold. This can be represented to the user.
    ///
    /// Value Type: dBFS
    pub post_expander: f32,

    /// The current full microphone amplitude (behaviour if compressor was disabled).
    ///
    /// Value Type: dBFS
    pub pre_compressor: f32,

    /// The microphone amplitude after compression has been applied.
    ///
    /// Value Type: dBFS
    pub post_compressor: f32,

    /// Unknown field, if anyone can make it not be 0, let me know!
    ///
    /// Value Type: Unknown
    pub float_12: f32,

    /// The left headphone meter.
    ///
    /// Value Type: dBFS
    pub hp_meter_left: f32,

    /// The right headphone meter.
    ///
    /// Value Type: dBFS
    pub hp_meter_right: f32,

    /// A clip warning. The value goes immediately to 1.0 if `raw_amplitude` goes above 0dBFS, then
    /// decays over roughly a quarter of a second.
    ///
    /// Value Type: Unknown
    pub clip_warning: f32,
}

// 16 floats here too, but these are all floats.
#[derive(Default, Debug, Copy, Clone)]
pub struct SuppressionResponse {
    pub float_0: f32,
    pub float_1: f32,
    pub float_2: f32,
    pub float_3: f32,
    pub float_4: f32,
    pub float_5: f32,
    pub float_6: f32,
    pub float_7: f32,
    pub float_8: f32,
    pub float_9: f32,
    pub float_10: f32,
    pub float_11: f32,
    pub float_12: f32,
    pub float_13: f32,
    pub float_14: f32,
    pub float_15: f32,
}

// Some values are linear in dB, others are raw dB, either way they're all dB so we're going
// to simply convert them.
fn lin_to_db(x: f32) -> f32 {
    if x <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * x.log10()
    }
}
