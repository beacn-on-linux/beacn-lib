use crate::controller::device::writer::UsbWriter;
use crate::timers::sleep;
use crate::types::RGBA;
use anyhow::Result;
use byteorder::{ByteOrder, LittleEndian};
use nusb::Interface;
use nusb::transfer::TransferError;
use web_time::{Duration, Instant};

pub struct Messenger {
    usb: UsbWriter,
    enabled: bool,
}

impl Messenger {
    pub(crate) fn new(interface: Interface, timeout: Duration) -> Result<Self> {
        Ok(Self {
            usb: UsbWriter::new(interface, timeout)?,
            enabled: false,
        })
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<(), TransferError> {
        self.usb.send(data).await
    }

    pub async fn send_timeout(
        &mut self,
        data: &[u8],
        timeout: Duration,
    ) -> Result<(), TransferError> {
        self.usb.send_timeout(data, timeout).await
    }

    pub async fn enable(&mut self, enabled: bool) -> Result<(), TransferError> {
        let value = if enabled { 0 } else { 1 };

        self.send(&[0, 1, 0, 4, value, 0, 0, 0]).await?;
        self.enabled = enabled;

        Ok(())
    }

    pub async fn ping(&mut self) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 0xf1]).await
    }

    pub async fn set_brightness(&mut self, brightness: u8) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 4, brightness, 0, 0, 0]).await
    }

    pub async fn set_button_brightness(&mut self, brightness: u8) -> Result<(), TransferError> {
        self.send(&[1, 7, 0, 4, brightness, 0, 0, 0]).await
    }

    pub async fn set_button_colour(
        &mut self,
        button: u8,
        colour: RGBA,
    ) -> Result<(), TransferError> {
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
        .await
    }

    pub async fn poll_inputs(&mut self) -> Result<(), TransferError> {
        self.send(&[0, 0, 0, 5]).await
    }

    pub async fn ensure_enabled(&mut self) -> Result<(), TransferError> {
        if !self.enabled {
            self.enable(true).await?;
            sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    pub(crate) fn build_image_chunks(x: u32, y: u32, img: &[u8]) -> Vec<[u8; 1024]> {
        let mut chunks = Vec::new();
        let mut iter = img.chunks(1020).enumerate().peekable();

        while let Some((index, value)) = iter.next() {
            let mut output = [0u8; 1024];

            LittleEndian::write_u24(&mut output[0..3], index as u32);
            output[3] = 0x50;
            output[4..4 + value.len()].copy_from_slice(value);

            chunks.push(output);

            if iter.peek().is_none() {
                let mut output = [0u8; 1024];

                output[0] = 0xff;
                output[1] = 0xff;
                output[2] = 0xff;
                output[3] = 0x50;

                LittleEndian::write_u32(&mut output[4..8], img.len() as u32 - 1);
                LittleEndian::write_u32(&mut output[8..12], x);
                LittleEndian::write_u32(&mut output[12..16], y);

                chunks.push(output);
            }
        }

        chunks
    }

    pub(crate) async fn send_chunk(
        &mut self,
        chunk: &[u8],
        retry: Duration,
    ) -> Result<(), TransferError> {
        let chunk_timeout = Duration::from_millis(100);

        let started = Instant::now();
        loop {
            match self.send_timeout(chunk, chunk_timeout).await {
                Ok(()) => return Ok(()),
                Err(TransferError::Cancelled) if started.elapsed() < retry => {
                    sleep(Duration::from_millis(20)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }
}
