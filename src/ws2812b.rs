use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Drive, SlewRate};
use embassy_rp::peripherals;
use embassy_rp::pio;
use embassy_rp::Peripheral;
use embassy_time::{Instant, Timer};
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;

use crate::board::LedsPio;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<peripherals::PIO0>;
});

pub struct WS2812B {
    pio: pio::Pio<'static, peripherals::PIO0>,
    last_write: Instant,
}

impl WS2812B {
    pub fn new(
        pio_per: impl Peripheral<P = LedsPio> + 'static,
        pin: impl pio::PioPin + 'static,
    ) -> Self {
        let mut pio = pio::Pio::new(pio_per, Irqs);
        let mut out_pin = pio.common.make_pio_pin(pin);
        out_pin.set_drive_strength(Drive::_8mA);
        out_pin.set_slew_rate(SlewRate::Fast);
        let prg = pio_proc::pio_file!("src/ws2812b.pio", select_program("ws2812b"));
        let mut cfg = pio::Config::default();
        cfg.use_program(&pio.common.load_program(&prg.program), &[&out_pin]);
        cfg.clock_divider = (U56F8!(125_000_000) / U56F8!(20_000_000)).to_fixed();
        cfg.shift_out.auto_fill = true;
        cfg.shift_out.threshold = 24;
        cfg.shift_out.direction = pio::ShiftDirection::Left;
        cfg.fifo_join = pio::FifoJoin::TxOnly;
        pio.sm0.set_pin_dirs(pio::Direction::Out, &[&out_pin]);
        pio.sm0.set_config(&cfg);
        pio.sm0.set_enable(true);

        Self {
            pio,
            last_write: Instant::now(),
        }
    }
    pub async fn write(&mut self, colors: &[u32]) {
        let elapsed = self.last_write.elapsed().as_micros();
        // minimum break 50us, but flushing the FIFO may take over 250us
        if elapsed < 320 {
            Timer::after_micros(320 - elapsed).await;
        }
        let tx = self.pio.sm0.tx();
        for rgb in colors {
            let grb = (rgb & 0xff0000) | ((rgb & 0x00ff00) << 16) | ((rgb & 0x0000ff) << 8);
            // let grb = grb >> 8;
            tx.wait_push(grb).await;
        }

        self.last_write = Instant::now();
    }
}
