use defmt::Format;
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, NoopRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};

use crate::config::MIDI_CHANNEL_SIZE;

#[derive(Clone, Copy, Format, PartialEq)]
pub enum MidiMsg {
    NoteOn { note: i8, velocity: i8 },
    NoteOff { note: i8, velocity: i8 },
}

impl MidiMsg {
    pub fn serialize(&self, buf: &mut [u8]) -> usize {
        match self {
            MidiMsg::NoteOn { note, velocity } => {
                if *note < 0 || *velocity < 0 || buf.len() < 3 {
                    0
                } else {
                    buf[0] = 0x90u8;
                    buf[1] = *note as u8;
                    buf[2] = *velocity as u8;
                    3
                }
            }
            MidiMsg::NoteOff { note, velocity } => {
                if *note < 0 || *velocity < 0 || buf.len() < 3 {
                    0
                } else {
                    buf[0] = 0x80u8;
                    buf[1] = *note as u8;
                    buf[2] = *velocity as u8;
                    3
                }
            }
        }
    }

    pub async fn send_bytes(
        &self,
        writer: &mut impl embedded_io_async::Write,
    ) -> Result<usize, &'static str> {
        let mut buf = [0u8; 3];

        let num_bytes = self.serialize(&mut buf);

        if num_bytes > 0 {
            writer
                .write_all(&buf[0..num_bytes])
                .await
                .map_err(|_| "io error")?
        }
        Ok(num_bytes)
    }

    pub fn usb_cin(&self) -> u8 {
        match self {
            MidiMsg::NoteOff { .. } => 0x08,
            MidiMsg::NoteOn { .. } => 0x09,
        }
    }
}

pub type MidiChannel = Channel<NoopRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelReceiver<'ch> = Receiver<'ch, NoopRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;

pub type MidiChannelMC = Channel<CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelMCReceiver<'ch> =
    Receiver<'ch, CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelMCSender<'ch> =
    Sender<'ch, CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
