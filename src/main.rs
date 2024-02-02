//! This example test the RP Pico on board LED.
//!
//! It does not work with the RP Pico W board. See wifi_blinky.rs.

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::gpio;
use embassy_time::{Duration, Timer};
use gpio::{Flex, Level, Output};
use {defmt_rtt as _, panic_probe as _};

mod button;
mod config;
mod touch_sensors;
mod ws2812b;

use crate::button::Button;
use crate::config::*;
use crate::touch_sensors::{CalibrationStatus, TouchSensors};
use crate::ws2812b::WS2812B;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut led = Output::new(p.PIN_25, Level::Low);

    info!("Minimal duration: {}ms", Duration::MIN.as_micros());

    info!("led on!");
    led.set_high();

    //let mut output_pin = Output::new(p.PIN_22, Level::Low);

    let leds = WS2812B::new(p.PIO0, p.PIN_26);
    let sensors = TouchSensors::new([
        Flex::new(p.PIN_6),
        Flex::new(p.PIN_7),
        Flex::new(p.PIN_8),
        Flex::new(p.PIN_9),
        Flex::new(p.PIN_10),
        Flex::new(p.PIN_11),
        Flex::new(p.PIN_12),
        Flex::new(p.PIN_13),
        Flex::new(p.PIN_14),
        Flex::new(p.PIN_15),
        Flex::new(p.PIN_16),
        Flex::new(p.PIN_17),
        Flex::new(p.PIN_18),
        Flex::new(p.PIN_19),
        Flex::new(p.PIN_20),
        Flex::new(p.PIN_21),
    ]);
    let button = Button::new(p.PIN_5);

    let bt_task = button.task();
    let main_task = measure_task(leds, sensors, &button);

    join(main_task, bt_task).await;
}

async fn measure_task<'a>(mut leds: WS2812B, mut sensors: TouchSensors<'a>, button: &Button<'a>) {
    let mut colors = [0u32; NUM_LEDS];

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
        for c in &mut colors {
            *c = 0x020100;
        }
        leds.write(&colors).await;

        sensors.calibrate_start().await;
        let mut cycle = 0;
        while !button.was_pressed() {
            let calib = sensors.calibrate_step().await;
            info!("{}", calib);
            for (i, color) in colors.iter_mut().enumerate().take(NUM_LEDS) {
                let sensor_num = i as i32 - FIRST_SENSOR_LED;
                let status = if sensor_num >= 0 && sensor_num < (NUM_SENSORS as i32) {
                    calib.pins[sensor_num as usize].status
                } else {
                    CalibrationStatus::NA
                };

                *color = match status {
                    CalibrationStatus::NA => 0x020100,
                    CalibrationStatus::Ok => 0x000200,
                    CalibrationStatus::Bad => 0x040000,
                } * if i == cycle { 4 } else { 1 };
            }
            leds.write(&colors).await;
            cycle = (cycle + 1) % NUM_LEDS;
            Timer::after_millis(100).await;
        }
        let result = sensors.calibrate_stop().await;
        if result.status != CalibrationStatus::Ok {
            continue;
        }

        while !button.was_pressed() {
            sensors.run().await;
            Timer::after_millis(2000).await;
        }
    }
}
