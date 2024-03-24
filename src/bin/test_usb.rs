#![no_std]
#![no_main]

use defmt::{unreachable, *};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

use touch_keyboard::midi::{MidiChannel, MidiMsg};
use touch_keyboard::usb_midi::UsbMidi;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let b = touch_keyboard::board::init(p);

    info!("MIDI USB test...");

    info!("led on!");
    let mut led = b.led_out;
    led.set_high();

    let midi_channel = MidiChannel::new();
    let mut usb_midi = UsbMidi::new(b.core1.midi_usb, midi_channel.receiver());

    let send_task = async {
        loop {
            midi_channel
                .send(MidiMsg::NoteOn {
                    note: 60,
                    velocity: 64,
                })
                .await;
            Timer::after_millis(100).await;
            midi_channel
                .send(MidiMsg::NoteOff {
                    note: 60,
                    velocity: 0,
                })
                .await;
            Timer::after_millis(1000).await;
        }
    };

    join(usb_midi.task(), send_task).await;

    unreachable!();
}
