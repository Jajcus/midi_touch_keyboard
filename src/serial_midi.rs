use defmt::info;
use embassy_rp::uart::BufferedUartTx;
use static_cell::StaticCell;

use crate::board::{Irqs, MidiTxPin, MidiUart};
use crate::config::SERIAL_MIDI_BUF_LEN;
use crate::midi::MidiChannelReceiver;

pub struct SerialMidi<'d> {
    uart: embassy_rp::uart::BufferedUartTx<'d, MidiUart>,
    midi_rx: MidiChannelReceiver<'d>,
}

impl<'d> SerialMidi<'d> {
    pub fn new(uart: MidiUart, tx_pin: MidiTxPin, midi_rx: MidiChannelReceiver<'d>) -> Self {
        let mut config = embassy_rp::uart::Config::default();

        config.baudrate = 31250;
        config.data_bits = embassy_rp::uart::DataBits::DataBits8;
        config.stop_bits = embassy_rp::uart::StopBits::STOP1;
        config.parity = embassy_rp::uart::Parity::ParityNone;
        config.invert_tx = true;

        static BUF: StaticCell<[u8; SERIAL_MIDI_BUF_LEN]> = StaticCell::new();
        let buf = &mut BUF.init([0; SERIAL_MIDI_BUF_LEN])[..];

        let uart = BufferedUartTx::new(uart, Irqs, tx_pin, buf, config);

        Self { uart, midi_rx }
    }
    pub async fn task(&mut self) -> ! {
        loop {
            let msg = self.midi_rx.receive().await;
            if let Err(err) = msg.send_bytes(&mut self.uart).await {
                info!("Midi send error: {}", err);
            }
        }
    }
}
