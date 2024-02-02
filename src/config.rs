use embassy_time::Duration;

// constants used throughout the code

// sensors
pub const NUM_SENSORS: usize = 16;
pub const CALIBRATION_STEP_TIME: Duration = Duration::from_micros(5000);
pub const MIN_TIME_REQUIRED: Duration = Duration::from_micros(10);
pub const MIN_MARGIN_REQUIRED: Duration = Duration::from_micros(100);

// leds
pub const NUM_LEDS: usize = 9;
pub const FIRST_SENSOR_LED: i32 = -3;

pub const WELCOME_COLORS: [u32; NUM_LEDS] = [
    0x80000, 0x008000, 0x000080, 0x40000, 0x004000, 0x000040, 0x20000, 0x002000, 0x000020,
];

// button
pub const DEBOUNCE_TIME: Duration = Duration::from_millis(2);
