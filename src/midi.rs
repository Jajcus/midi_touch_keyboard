use defmt::Format;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, CriticalSectionRawMutex};
use embassy_sync::channel::{Channel, Receiver, Sender};

use crate::config::MIDI_CHANNEL_SIZE;

#[derive(Clone, Copy, Format, PartialEq)]
pub enum MidiMsg {
    NoteOn { note: i8, velocity: i8 },
    NoteOff { note: i8, velocity: i8 },
}

impl MidiMsg {
    pub async fn send_bytes(
        &self,
        writer: &mut impl embedded_io_async::Write,
    ) -> Result<usize, &'static str> {
        match self {
            MidiMsg::NoteOn { note, velocity } => {
                if *note < 0 || *velocity < 0 { return Err("invalid value") };
                let bytes = [0x90u8, *note as u8, *velocity as u8];
                writer.write(&bytes).await.map_err(|_| "io error")?;
                Ok(3)
            }
            MidiMsg::NoteOff { note, velocity } => {
                if *note < 0 || *velocity < 0 { return Err("invalid value") };
                let bytes = [0x80u8, *note as u8, *velocity as u8];
                writer.write(&bytes).await.map_err(|_| "io error")?;
                Ok(3)
            }
        }
    }
}

pub type MidiChannel = Channel<NoopRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelReceiver<'ch> = Receiver<'ch, NoopRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
#[allow(dead_code)]
pub type MidiChannelSender<'ch> = Sender<'ch, NoopRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;

pub type MidiChannelMC = Channel<CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelMCReceiver<'ch> = Receiver<'ch, CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
pub type MidiChannelMCSender<'ch> = Sender<'ch, CriticalSectionRawMutex, MidiMsg, MIDI_CHANNEL_SIZE>;
