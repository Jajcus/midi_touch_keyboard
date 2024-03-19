//! Sets all pins to high-impedance input state and blinks LED rapidly
//! usefull for troubleshooting the circuit (so the external 10Mohm pull-downs are measurable)

#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_time::Timer;
use gpio::{Input, Level, Output, Pull};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    info!("Setting inputs to high-Z");

    let p0 = Input::new(p.PIN_0, Pull::None);
    let p1 = Input::new(p.PIN_1, Pull::None);
    let p2 = Input::new(p.PIN_2, Pull::None);
    let p3 = Input::new(p.PIN_3, Pull::None);
    let p4 = Input::new(p.PIN_4, Pull::None);
    let p5 = Input::new(p.PIN_5, Pull::None);
    let p6 = Input::new(p.PIN_6, Pull::None);
    let p7 = Input::new(p.PIN_7, Pull::None);
    let p8 = Input::new(p.PIN_8, Pull::None);
    let p9 = Input::new(p.PIN_9, Pull::None);
    let p10 = Input::new(p.PIN_10, Pull::None);
    let p11 = Input::new(p.PIN_11, Pull::None);
    let p12 = Input::new(p.PIN_12, Pull::None);
    let p13 = Input::new(p.PIN_13, Pull::None);
    let p14 = Input::new(p.PIN_14, Pull::None);
    let p15 = Input::new(p.PIN_15, Pull::None);
    let p16 = Input::new(p.PIN_16, Pull::None);
    let p17 = Input::new(p.PIN_17, Pull::None);
    let p18 = Input::new(p.PIN_18, Pull::None);
    let p19 = Input::new(p.PIN_19, Pull::None);
    let p20 = Input::new(p.PIN_20, Pull::None);
    let p21 = Input::new(p.PIN_21, Pull::None);
    let p22 = Input::new(p.PIN_22, Pull::None);

    _ = (
        p0, p1, p2, p3, p4, p5, p6, p7, p8, p9, p10, p11, p12, p13, p14, p15, p16, p17, p18, p19,
        p20, p21, p22,
    );

    let mut led = Output::new(p.PIN_25, Level::Low);
    loop {
        led.set_high();
        Timer::after_millis(100).await;

        led.set_low();
        Timer::after_millis(100).await;
    }
}
