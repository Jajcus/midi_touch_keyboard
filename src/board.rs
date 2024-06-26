use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Flex, Input, Level, Output, Pull};
use embassy_rp::peripherals::*;
use embassy_rp::Peripherals;

use crate::config::NUM_SENSORS;

pub type LedsPio = PIO0;
pub type LedsPin = PIN_27;

pub type AdcPins = (PIN_28, PIN_29);

pub type MidiUart = UART0;
pub type MidiTxPin = PIN_0;

pub type MidiUsb = USB;

bind_interrupts!(pub struct Irqs {
    UART0_IRQ => embassy_rp::uart::BufferedInterruptHandler<MidiUart>;
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
});

pub struct Core0Pers {
    pub button_in: Input<'static>,

    pub leds_pio: LedsPio,
    pub leds_pin: LedsPin,

    pub sensor_pins: [Flex<'static>; NUM_SENSORS],

    pub adc: ADC,
    pub adc_pins: AdcPins,
}

pub struct Core1Pers {
    pub midi_uart: MidiUart,
    pub midi_tx_pin: MidiTxPin,
    pub midi_usb: MidiUsb,
}

pub struct BoardSetup {
    pub led_out: Output<'static>,

    pub core0: Core0Pers,

    pub core1: Core1Pers,
    pub core1_core: CORE1,
}

pub fn init(p: Peripherals) -> BoardSetup {
    BoardSetup {
        led_out: Output::new(p.PIN_25, Level::Low),

        core0: Core0Pers {
            button_in: Input::new(p.PIN_4, Pull::Up),

            leds_pio: p.PIO0,
            leds_pin: p.PIN_27,

            sensor_pins: [
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
            ],
            adc: p.ADC,
            adc_pins: (p.PIN_28, p.PIN_29),
        },
        core1: Core1Pers {
            midi_uart: p.UART0,
            midi_tx_pin: p.PIN_0,
            midi_usb: p.USB,
        },
        core1_core: p.CORE1,
    }
}
