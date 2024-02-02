// Debounced button

use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_rp::Peripheral;
use embassy_time::{Duration, Timer};

use core::cell::{Cell, RefCell};

const DEBOUNCE_TIME: Duration = Duration::from_millis(2);

pub struct Button<'a> {
    pin: RefCell<Input<'a>>,
    pressed: Cell<bool>,
    pressed_was: Cell<bool>,
}

impl<'a> Button<'a> {
    pub fn new(pin: impl Peripheral<P = impl Pin> + 'a) -> Self {
        let mut pin = Input::new(pin, Pull::Up);
        pin.set_schmitt(true);
        Self {
            pin: pin.into(),
            pressed: false.into(),
            pressed_was: false.into(),
        }
    }
    #[allow(dead_code)]
    pub fn is_pressed(&self) -> bool {
        self.pressed.get()
    }
    pub fn was_pressed(&self) -> bool {
        self.pressed_was.replace(false)
    }
    #[allow(clippy::await_holding_refcell_ref)]
    pub async fn task(&self) -> ! {
        let mut pin = self.pin.borrow_mut();
        let mut pressed = pin.is_low();
        self.pressed.set(pressed);
        loop {
            if pressed {
                pin.wait_for_high().await;
            } else {
                pin.wait_for_low().await;
            }
            Timer::after(DEBOUNCE_TIME).await;
            let now_low = pin.is_low();
            if now_low != pressed {
                pressed = now_low;
                self.pressed.set(pressed);
                if now_low {
                    self.pressed_was.set(true);
                }
            }
        }
    }
}
