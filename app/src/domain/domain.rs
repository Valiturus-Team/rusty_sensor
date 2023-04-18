use std::io::{Read, Write};

pub trait ReadWrite: Write + Read {}

// blanket implementation:
impl<T: Write + Read> ReadWrite for T {}

// pub struct ButtonEvent(bool);

pub type ButtonEvent = bool;

// LSMData .
#[derive(Debug)]
pub struct LSMData {
    pub mag_x: i32,
    pub mag_y: i32,
    pub mag_z: i32,
    pub accel_x: i32,
    pub accel_y: i32,
    pub accel_z: i32,
    pub timestamp: i64,
}

// BMI160Data .
#[derive(Debug)]
pub struct BMI160Data {
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub sample_time: u32,
}
