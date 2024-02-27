use embassy_rp::bind_interrupts;
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::ADC;
use embassy_rp::Peripheral;
use embassy_time::Timer;

use core::cell::Cell;

use crate::board::AdcPins;
use crate::config::ADC_INTERVAL;

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
});

pub struct AdcValues {
    values: [Cell<u32>; 2],
}

pub struct Adc<'d> {
    adc: embassy_rp::adc::Adc<'d, embassy_rp::adc::Async>,
    channels: [embassy_rp::adc::Channel<'d>; 2],
    values: &'d AdcValues,
}

impl<'d> Adc<'d> {
    pub fn new(
        adc_per: impl Peripheral<P = ADC> + 'd,
        pins: AdcPins,
        values: &'d AdcValues,
    ) -> Result<Self, &'static str> {
        let config = embassy_rp::adc::Config::default();
        let adc = embassy_rp::adc::Adc::new(adc_per, Irqs, config);

        let channels = [
            embassy_rp::adc::Channel::new_pin(pins.0, Pull::None),
            embassy_rp::adc::Channel::new_pin(pins.1, Pull::None),
        ];

        Ok(Self {
            adc,
            channels,
            values,
        })
    }
    pub async fn task(mut self) -> ! {
        loop {
            for (i, ch) in self.channels.iter_mut().enumerate() {
                let value = self.adc.read(ch).await.unwrap_or(0).into();
                self.values.values[i].set(value);
            }
            Timer::after(ADC_INTERVAL).await;
        }
    }
}

impl AdcValues {
    pub fn new() -> Self {
        Self {
            values: core::array::from_fn(|_| 0.into()),
        }
    }
    pub fn get_value(&self, input_number: usize, scale: u32) -> Result<u32, &'static str> {
        if input_number >= 2 {
            Err("Wrong input number")
        } else {
            let value = self.values[input_number].get();
            Ok(value * (scale + 1) / 4096)
        }
    }
}
