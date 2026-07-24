use crate::controller::device::writer::UsbWriter;
use crate::types::RGBA;
use byteorder::{ByteOrder, LittleEndian};
use log::error;
use nusb::transfer::{Interrupt, Out, TransferError};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct Messenger<'a> {
    usb: UsbWriter<'a>,
    enabled: bool,
}

impl<'a> Messenger<'a> {
    pub(crate) fn new(endpoint: &'a mut nusb::Endpoint<Interrupt, Out>, timeout: Duration) -> Self {
        Self {
            usb: UsbWriter::new(endpoint, timeout),
            enabled: false,
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.usb.send(data)
    }

    pub fn enable(&mut self, enabled: bool) -> Result<(), TransferError> {
        let value = if enabled { 0 } else { 1 };

        self.send(&[0, 1, 0, 4, value, 0, 0, 0])?;
        self.enabled = enabled;

        Ok(())
    }

    pub fn ping(&mut self) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 0xf1])
    }

    pub fn set_screen_brightness(&mut self, brightness: u8) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 4, brightness, 0, 0, 0])
    }

    pub fn set_button_brightness(&mut self, brightness: u8) -> Result<(), TransferError> {
        self.send(&[1, 7, 0, 4, brightness, 0, 0, 0])
    }

    pub fn set_button_colour(&mut self, button: u8, colour: RGBA) -> Result<(), TransferError> {
        self.send(&[
            1,
            button,
            0,
            4,
            colour.blue,
            colour.green,
            colour.red,
            colour.alpha,
        ])
    }

    pub fn poll_inputs(&mut self) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 5])
    }

    pub fn ensure_enabled(&mut self) -> Result<(), TransferError> {
        if !self.enabled {
            self.enable(true)?;
            sleep(Duration::from_millis(100));
        }
        Ok(())
    }

    pub fn send_image(&mut self, x: u32, y: u32, img: &[u8]) -> Result<(), TransferError> {
        let overall_budget = Duration::from_secs(10);
        let overall_started = Instant::now();

        while overall_started.elapsed() < overall_budget {
            match self.send_image_attempt(x, y, img) {
                Ok(()) => {
                    sleep(Duration::from_millis(10));
                    return Ok(());
                }

                Err(TransferError::Cancelled) => {
                    sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }

        error!(
            "Failed to send image after {:?}, dropping frame.",
            overall_started.elapsed()
        );

        Err(TransferError::Cancelled)
    }

    fn send_image_attempt(&mut self, x: u32, y: u32, img: &[u8]) -> Result<(), TransferError> {
        let chunk_retry = Duration::from_millis(300);

        let mut output = [0u8; 1024];
        let mut iter = img.chunks(1020).enumerate().peekable();

        while let Some((index, value)) = iter.next() {
            output.fill(0);

            LittleEndian::write_u24(&mut output[0..3], index as u32);
            output[3] = 0x50;
            output[4..4 + value.len()].copy_from_slice(value);

            self.send_chunk(&output, chunk_retry)?;

            if iter.peek().is_none() {
                output.fill(0);

                output[0] = 0xff;
                output[1] = 0xff;
                output[2] = 0xff;
                output[3] = 0x50;

                LittleEndian::write_u32(&mut output[4..8], img.len() as u32 - 1);
                LittleEndian::write_u32(&mut output[8..12], x);
                LittleEndian::write_u32(&mut output[12..16], y);

                self.send_chunk(&output, chunk_retry)?;
            }
        }

        Ok(())
    }

    fn send_chunk(&mut self, chunk: &[u8; 1024], retry: Duration) -> Result<(), TransferError> {
        let started = Instant::now();
        loop {
            match self.send(chunk) {
                Ok(()) => return Ok(()),
                Err(TransferError::Cancelled) if started.elapsed() < retry => {
                    sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(e),
            }
        }
    }
}
