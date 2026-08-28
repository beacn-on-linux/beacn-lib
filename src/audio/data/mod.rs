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
        match self {
            BulkMessage::GetMeters => true,
            BulkMessage::GetSuppressionBase => true,
            BulkMessage::GetSuppressionCurrent => true,
            _ => false,
        }
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
                // Ok, we have 16 maybe floats here (4, 12, 13, 14 may be u32), so just label them for now,
                // we'll correct them later
                let response = MetersResponse {
                    float_0: cursor.read_f32::<LittleEndian>()?,
                    float_1: cursor.read_f32::<LittleEndian>()?,
                    float_2: cursor.read_f32::<LittleEndian>()?,
                    float_4: cursor.read_f32::<LittleEndian>()?,
                    float_5: cursor.read_f32::<LittleEndian>()?,
                    float_6: cursor.read_f32::<LittleEndian>()?,
                    float_7: cursor.read_f32::<LittleEndian>()?,
                    float_8: cursor.read_f32::<LittleEndian>()?,
                    float_9: cursor.read_f32::<LittleEndian>()?,
                    float_10: cursor.read_f32::<LittleEndian>()?,
                    float_11: cursor.read_f32::<LittleEndian>()?,
                    float_12: cursor.read_f32::<LittleEndian>()?,
                    float_13: cursor.read_f32::<LittleEndian>()?,
                    float_14: cursor.read_f32::<LittleEndian>()?,
                    float_15: cursor.read_f32::<LittleEndian>()?,
                };

                BulkMessage::Meters(response)
            }

            BulkMessage::GetSuppressionBase | BulkMessage::GetSuppressionCurrent => {
                // 16 floats here too,
                let response = SuppressionResponse {
                    float_0: cursor.read_f32::<LittleEndian>()?,
                    float_1: cursor.read_f32::<LittleEndian>()?,
                    float_2: cursor.read_f32::<LittleEndian>()?,
                    float_3: cursor.read_f32::<LittleEndian>()?,
                    float_4: cursor.read_f32::<LittleEndian>()?,
                    float_5: cursor.read_f32::<LittleEndian>()?,
                    float_6: cursor.read_f32::<LittleEndian>()?,
                    float_7: cursor.read_f32::<LittleEndian>()?,
                    float_8: cursor.read_f32::<LittleEndian>()?,
                    float_9: cursor.read_f32::<LittleEndian>()?,
                    float_10: cursor.read_f32::<LittleEndian>()?,
                    float_11: cursor.read_f32::<LittleEndian>()?,
                    float_12: cursor.read_f32::<LittleEndian>()?,
                    float_13: cursor.read_f32::<LittleEndian>()?,
                    float_14: cursor.read_f32::<LittleEndian>()?,
                    float_15: cursor.read_f32::<LittleEndian>()?,
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

// Ok, we have 16 maybe floats here (4, 12, 13, 14 may be u32), so just label them for now,
// we'll correct them later once we can confirm their behaviours.
#[derive(Debug, Copy, Clone)]
pub struct MetersResponse {
    pub float_0: f32,
    pub float_1: f32,
    pub float_2: f32,
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

    // This float is common across all responses, probable activity level
    pub float_15: f32,
}

// 16 floats here too, but these are all floats.
#[derive(Debug, Copy, Clone)]
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
