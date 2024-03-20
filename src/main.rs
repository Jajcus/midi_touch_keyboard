//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See wifi_blinky.rs.

#![no_std]
#![no_main]

use defmt::{unreachable, *};
use embassy_futures::join::{join3, join};
use embassy_time::{Duration, Timer};
use embassy_executor::Executor;
use {defmt_rtt as _, panic_probe as _};
use embassy_rp::multicore::{spawn_core1, Stack};
use static_cell::StaticCell;

mod adc;
mod board;
mod button;
mod config;
mod midi;
mod serial_midi;
mod touch_sensors;
mod ws2812b;

use crate::adc::{Adc, AdcValues};
use crate::button::Button;
use crate::config::*;
use crate::midi::{MidiChannel, MidiChannelMC, MidiChannelMCSender, MidiChannelMCReceiver, MidiMsg};
use crate::serial_midi::SerialMidi;
use crate::touch_sensors::{CalibrationStatus, TouchSensorStatus, TouchSensors};
use crate::ws2812b::WS2812B;

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static mut CORE1_STACK: Stack<81920> = Stack::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    let b = crate::board::init(p);

    let mut led = b.led_out;

    info!("Minimal duration: {}ms", Duration::MIN.as_micros());

    info!("led on!");
    led.set_high();

    static MIDI_CHANNEL: MidiChannelMC = MidiChannelMC::new();

    spawn_core1(b.core1_core, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1
            .run(|spawner| unwrap!(spawner.spawn(core1_task(b.core1, MIDI_CHANNEL.receiver()))))
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        unwrap!(spawner.spawn(core0_task(b.core0, MIDI_CHANNEL.sender())))
    })
}

#[embassy_executor::task]
async fn core0_task(b: crate::board::Core0Pers, midi_tx: MidiChannelMCSender<'static>) -> ! {

    let leds = WS2812B::new(b.leds_pio, b.leds_pin);
    let sensors = TouchSensors::new(b.sensor_pins);

    let adc_values = AdcValues::new();
    let button = Button::new(b.button_in);
    let adc = Adc::new(b.adc, b.adc_pins, &adc_values).unwrap();

    let bt_task = button.task();
    let adc_task = adc.task();

    let main_task = measure_task(leds, sensors, &button, &adc_values, midi_tx);

    join3(main_task, bt_task, adc_task).await;

    unreachable!();
}

#[embassy_executor::task]
async fn core1_task(b: crate::board::Core1Pers, midi_c0_rx: MidiChannelMCReceiver<'static>) -> ! {

    let midi_channel = MidiChannel::new();
    let midi_tx = midi_channel.sender();
    let mut midi = SerialMidi::new(b.midi_uart, b.midi_tx_pin, midi_channel.receiver());
    let midi_task = midi.task();

    let midi_router_task = async {
        loop {
            let msg = midi_c0_rx.receive().await;
            midi_tx.send(msg).await;
        }
    };

    join(midi_router_task, midi_task).await;

    unreachable!();
}

async fn measure_task<'a>(
    mut leds: WS2812B,
    mut sensors: TouchSensors<'a>,
    button: &Button<'a>,
    adc_values: &'a AdcValues,
    midi_tx: MidiChannelMCSender<'a>,
) {
    let mut colors = [COL_UNUSED; NUM_LEDS];

    for i in 0..NUM_LEDS {
        colors[i] = WELCOME_COLORS[i];
        leds.write(&colors).await;
        Timer::after_millis(50).await;
    }
    for i in 0..NUM_LEDS {
        colors[i] = 0;
        leds.write(&colors).await;
        Timer::after_millis(50).await;
    }

    loop {
        colors = [COL_CAL_NA; NUM_LEDS];
        leds.write(&colors).await;

        sensors.calibrate_start().await;
        let mut cycle = 0;
        while !button.was_pressed() {
            let calib = sensors.calibrate_step().await;
            info!("{}", calib);
            info!("sens: {}%", adc_values.get_value(0, 100));
            for (i, which_led) in SENSOR_TO_LED.iter().enumerate().take(NUM_SENSORS) {
                let color = match which_led {
                    None => {
                        continue;
                    }
                    Some(led) => &mut colors[*led],
                };
                let status = calib.pins[i].status;
                *color = match status {
                    CalibrationStatus::NA => COL_CAL_NA,
                    CalibrationStatus::Ok => COL_CAL_OK,
                    CalibrationStatus::Bad => COL_CAL_BAD,
                } * if i == cycle { 4 } else { 1 };
            }
            leds.write(&colors).await;
            cycle = (cycle + 1) % NUM_SENSORS;
            Timer::after_millis(100).await;
        }
        let result = sensors.calibrate_stop().await;
        if result.status != CalibrationStatus::Ok {
            continue;
        }

        colors = [COL_UNUSED; NUM_LEDS];
        for (i, which_led) in SENSOR_TO_LED.iter().enumerate().take(NUM_SENSORS) {
            let color = match which_led {
                None => {
                    continue;
                }
                Some(led) => &mut colors[*led],
            };
            let status = result.pins[i].status;
            let piano_key = SENSOR_TO_PIANO_KEY[i];
            *color = match (status, piano_key) {
                (CalibrationStatus::NA, PianoKey::White) => COL_WHITE_OFF,
                (CalibrationStatus::NA, PianoKey::Black) => COL_BLACK_OFF,
                (CalibrationStatus::Ok, PianoKey::White) => COL_WHITE_OFF,
                (CalibrationStatus::Ok, PianoKey::Black) => COL_BLACK_OFF,
                (CalibrationStatus::Bad, _) => COL_BROKEN,
                (_, PianoKey::Missing) => COL_UNUSED,
            }
        }
        leds.write(&colors).await;

        let mut prev_status = [TouchSensorStatus::NA; NUM_SENSORS];
        while !button.was_pressed() {
            let sens = adc_values.get_value(0, 1000).unwrap_or(500);
            sensors.set_sensitivity(sens);
            let status = sensors.run().await;
            debug!("Status: {}", status);
            debug!("Previous: {}", prev_status);
            for (i, (prev, cur)) in prev_status.iter_mut().zip(status.iter()).enumerate() {
                if *cur == *prev {
                    continue;
                }
                match *cur {
                    TouchSensorStatus::NA => continue,
                    TouchSensorStatus::On => {
                        info!("Touched: {}", i);
                    }
                    TouchSensorStatus::Off => {
                        info!("Released: {}", i);
                    }
                };
                *prev = *cur;
                let piano_key = SENSOR_TO_PIANO_KEY[i];
                let color = match (*cur, piano_key) {
                    (TouchSensorStatus::Off, PianoKey::White) => COL_WHITE_OFF,
                    (TouchSensorStatus::Off, PianoKey::Black) => COL_BLACK_OFF,
                    (TouchSensorStatus::On, PianoKey::White) => COL_WHITE_ON,
                    (TouchSensorStatus::On, PianoKey::Black) => COL_BLACK_ON,
                    (_, PianoKey::Missing) => COL_UNUSED,
                    _ => unreachable!(),
                };
                let maybe_note_nr = if let Some(nr) = SENSOR_TO_NOTE[i] {
                    let note_nr = ROOT_NOTE + nr;
                    if note_nr >= 0 {
                        Some(note_nr)
                    } else {
                        None
                    }
                } else {
                    None
                };

                match (*cur, maybe_note_nr) {
                    (TouchSensorStatus::On, Some(note_nr)) => {
                        let msg = MidiMsg::NoteOn {
                            note: note_nr,
                            velocity: 64,
                        };
                        info!("Midi: {}", msg);
                        midi_tx.try_send(msg).ok(); // ignore error (buffer full)
                    }
                    (TouchSensorStatus::Off, Some(note_nr)) => {
                        let msg = MidiMsg::NoteOff {
                            note: note_nr,
                            velocity: 0,
                        };
                        info!("Midi: {}", msg);
                        midi_tx.send(msg).await; // wait until this note-off can be sent
                    }
                    _ => (),
                };
                if let Some(led) = SENSOR_TO_LED[i] {
                    colors[led] = color;
                    leds.write(&colors).await;
                }
            }
            Timer::after_millis(2).await;
        }
    }
}
