use defmt::{info, write, Format, Formatter};

use embassy_rp::gpio::{Flex, Pull};
use embassy_time::{Duration, Instant, Timer};

use crate::config::*;

#[derive(Default, Clone, Copy, Format, PartialEq)]
pub enum CalibrationStatus {
    #[default]
    NA,
    Ok,
    Bad,
}

#[derive(Clone, Copy)]
pub struct CalibrationData {
    pub min_time: Duration,
    pub max_time: Duration,
    pub status: CalibrationStatus,
}

impl Default for CalibrationData {
    fn default() -> Self {
        Self {
            min_time: Duration::MAX,
            max_time: Duration::MIN,
            status: CalibrationStatus::NA,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct CalibrationDataSet {
    pub pins: [CalibrationData; NUM_SENSORS],
    pub all: CalibrationData,
    pub status: CalibrationStatus,
}

impl Format for CalibrationDataSet {
    fn format(&self, f: Formatter) {
        write!(f, "{}: ", self.status);
        let mut first = true;
        for pin_data in self.pins {
            if !first {
                write!(f, ", ");
            } else {
                first = false;
            }
            write!(f, "{}[", pin_data.status);
            if pin_data.min_time < Duration::MAX {
                write!(f, "{}", pin_data.min_time.as_micros());
            }
            write!(f, "..");
            if pin_data.max_time > Duration::MIN {
                write!(f, "{}", pin_data.max_time.as_micros());
            }
            write!(f, "]");
        }
    }
}

pub struct TouchSensors<'a> {
    pins: [Flex<'a>; NUM_SENSORS],
    calibration: CalibrationDataSet,
    threshold: Duration,
}

impl<'a> TouchSensors<'a> {
    pub fn new(mut pins: [Flex<'a>; NUM_SENSORS]) -> Self {
        let threshold = Duration::from_micros(500);

        for pin in &mut pins {
            pin.set_pull(Pull::None);
        }

        Self {
            pins,
            calibration: Default::default(),
            threshold,
        }
    }
    pub async fn calibrate_start(&mut self) {
        info!("Calibration start");
        self.calibration = Default::default();
    }
    pub async fn calibrate_step(&mut self) -> CalibrationDataSet {
        info!("Calibration step");

        for pin in &mut self.pins {
            pin.set_as_output();
            pin.set_high();
        }

        Timer::after(Duration::from_micros(1)).await;

        let t0 = Instant::now();

        for pin in &mut self.pins {
            pin.set_as_input();
        }

        let mut already_done: [bool; NUM_SENSORS] = [false; NUM_SENSORS];
        let mut num_done = 0;
        let all_c = &mut self.calibration.all;
        loop {
            Timer::after(Duration::from_micros(1)).await;
            let t = Instant::elapsed(&t0);
            let done: [bool; NUM_SENSORS] = core::array::from_fn(|i| self.pins[i].is_low());

            for i in 0..NUM_SENSORS {
                if already_done[i] {
                    continue;
                }
                if done[i] {
                    let pin_c = &mut self.calibration.pins[i];
                    if t < pin_c.min_time {
                        pin_c.min_time = t;
                    }
                    if t > pin_c.max_time {
                        pin_c.max_time = t;
                    }
                    if pin_c.min_time < MIN_TIME_REQUIRED {
                        pin_c.status = CalibrationStatus::Bad;
                    } else if pin_c.max_time - pin_c.min_time >= MIN_MARGIN_REQUIRED {
                        pin_c.status = CalibrationStatus::Ok;

                        // get those only from reasonable-behaving pins
                        if pin_c.min_time < all_c.min_time {
                            all_c.min_time = t;
                        }
                        if pin_c.max_time > all_c.max_time {
                            all_c.max_time = t;
                        }
                    }
                    already_done[i] = true;
                    num_done += 1;
                }
            }

            if all_c.max_time > all_c.min_time {
                // we have some usable data, use it to find some bad sensors
                for i in 0..NUM_SENSORS {
                    let pin_c = &mut self.calibration.pins[i];
                    if pin_c.min_time > all_c.max_time {
                        pin_c.status = CalibrationStatus::Bad;
                    }
                }
            }

            if num_done == NUM_SENSORS || t >= CALIBRATION_STEP_TIME {
                break;
            };
        }

        self.calibration
    }
    pub async fn calibrate_stop(&mut self) -> CalibrationDataSet {
        info!("Calibration end");
        let max_min = self
            .calibration
            .pins
            .iter()
            .filter(|x| x.status == CalibrationStatus::Ok)
            .map(|x| x.min_time)
            .max();
        let min_max = self
            .calibration
            .pins
            .iter()
            .filter(|x| x.status == CalibrationStatus::Ok)
            .map(|x| x.max_time)
            .min();

        match (max_min, min_max) {
            (Some(max_min), Some(min_max)) => {
                if min_max < max_min || (min_max - max_min) < MIN_MARGIN_REQUIRED / 10 {
                    info!(
                        "Bad calibration max min: {}  min max: {}",
                        max_min.as_micros(),
                        min_max.as_micros()
                    );
                    self.calibration.status = CalibrationStatus::Bad;
                    self.threshold =
                        self.calibration.all.min_time + self.calibration.all.max_time / 2;
                } else {
                    self.threshold = max_min + min_max / 2;
                };
                self.calibration.status = CalibrationStatus::Ok;
            }
            (_, _) => {
                info!("Bad calibration no single good pin");
                self.threshold = CALIBRATION_STEP_TIME / 2;
                self.calibration.status = CalibrationStatus::Bad;
            }
        }
        info!(
            "status: {}, threshold: {}",
            self.calibration.status,
            self.threshold.as_micros()
        );
        self.calibration
    }
    pub async fn run(&mut self) {
        for pin in &mut self.pins {
            pin.set_as_output();
            pin.set_high();
        }

        info!("set high!");
        Timer::after(Duration::from_micros(1)).await;

        let t0 = Instant::now();

        for pin in &mut self.pins {
            pin.set_as_input();
        }

        Timer::at(t0 + MIN_TIME_REQUIRED).await;

        let early: [bool; NUM_SENSORS] = core::array::from_fn(|i| self.pins[i].is_low());

        Timer::at(t0 + self.threshold).await;

        let off: [bool; NUM_SENSORS] = core::array::from_fn(|i| self.pins[i].is_low());

        //Timer::at(t0 + self.time_max).await;
        //
        //let late: [bool;NUM_SENSORS] = core::array::from_fn(|i| self.pins[i].is_low());

        for i in 0..NUM_SENSORS {
            if early[i] {
                info!("PIN {}  early", i)
            } else if off[i] {
                info!("PIN {}  OFF", i)
            } else {
                info!("PIN {} ON", i)
            }
        }
    }
}
