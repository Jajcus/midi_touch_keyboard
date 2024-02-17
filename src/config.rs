use embassy_time::Duration;

// constants used throughout the code

// sensors
pub const NUM_SENSORS: usize = 16;
pub const SENSOR_SAMPLES: usize = 10;
pub const CALIBRATION_STEP_TIME: Duration = Duration::from_micros(5000);
pub const MIN_TIME_REQUIRED: Duration = Duration::from_micros(10);
pub const MIN_MARGIN_REQUIRED: Duration = Duration::from_micros(100);

// leds
pub const NUM_LEDS: usize = 19;
pub const WELCOME_COLORS: [u32; NUM_LEDS] = [
    0x80000, 0x008000, 0x000080, 0x40000, 0x004000, 0x000040, 0x20000, 0x002000, 0x000020,
    0x80000, 0x008000, 0x000080, 0x40000, 0x004000, 0x000040, 0x20000, 0x002000, 0x000020,
    0x10000
];

pub const COL_OFF: u32 = 0x020202;
pub const COL_ON: u32 = 0x101010;
pub const COL_BROKEN: u32 = 0x010000;
pub const COL_UNUSED: u32 = 0x000000;

// must be still valid (just brighter) when multiplied by 4
pub const COL_CAL_NA: u32 = 0x020100;
pub const COL_CAL_OK: u32 = 0x000200;
pub const COL_CAL_BAD: u32 = 0x040000;

pub const SENSOR_TO_LED: [Option<usize>; NUM_SENSORS] = [
    Some(9),
    Some(8),
    Some(11),
    Some(7),
    Some(12),
    Some(6),
    Some(5),
    Some(14),
    Some(4),
    Some(15),
    Some(3),
    Some(16),
    Some(2),
    Some(1),
    Some(18),
    Some(0),
];

// button
pub const DEBOUNCE_TIME: Duration = Duration::from_millis(2);
