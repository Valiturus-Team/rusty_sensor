use std::{
    io::Write,
    sync::{self, Arc},
    thread, time,
};

use crate::domain::domain;
use crate::{domain::domain::ButtonEvent, rust_proto::algorithim};
use crossbeam_channel::Receiver;
use esp_idf_hal::task::thread::ThreadSpawnConfiguration;
use protobuf::Message;

pub struct App {
    sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
    sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
    algorithim_configuration: algorithim::AlgorithimConfiguration,
    button_events: Receiver<domain::ButtonEvent>,
    lsm_events: Receiver<domain::LSMData>,
    bmi_events: Receiver<domain::BMI160Data>,
}

impl App {
    pub fn new(
        sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
        sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
        button_eventer: Receiver<ButtonEvent>,
        lsm_events: Receiver<domain::LSMData>,
        bmi_events: Receiver<domain::BMI160Data>,
    ) -> Self {
        App {
            sensor_input_buffer,
            sensor_output_buffer,
            algorithim_configuration: algorithim::AlgorithimConfiguration::default(),
            button_events: button_eventer,
            bmi_events,
            lsm_events,
        }
    }
    // write data to buffer
    fn write_data(self, bytes: &Vec<u8>) {
        let out_buffer = Arc::clone(&self.sensor_output_buffer);
        out_buffer.lock().unwrap().write_all(bytes).unwrap();
    }
    fn set_configuration(&mut self, conf: algorithim::AlgorithimConfiguration) {
        self.algorithim_configuration = conf;
    }
    pub fn run(mut self) {
        let bmi_events = self.bmi_events.clone();
        let lsm_events = self.lsm_events.clone();
        // button event thread
        let btn_events = self.button_events.clone();

        let inp_buffer = Arc::clone(&self.sensor_input_buffer);

        // configure thread
        ThreadSpawnConfiguration {
            name: Some("Thread-D\0".as_bytes()),
            priority: 5,
            ..Default::default()
        }
        .set()
        .unwrap();
        let _ = thread::Builder::new().spawn(move || loop {
            let mut buffer = inp_buffer.lock().unwrap();

            if buffer.len() == 0 {
                thread::sleep(time::Duration::from_millis(50));
                continue;
            }

            let tmp_buffer = buffer.clone();
            buffer.clear();

            let test: Result<algorithim::Message, protobuf::Error> =
                Message::parse_from_bytes(&tmp_buffer);

            match test {
                Ok(message) => {
                    message.has_Algorithim();
                    if message.has_Algorithim() {
                        self.set_configuration(message.Algorithim().clone());
                    }
                }
                Err(_err) => {
                    ::log::info!("error parsing message {:?}", _err)
                }
            }
        });

        // configure thread
        ThreadSpawnConfiguration {
            name: Some("Thread-E\0".as_bytes()),
            priority: 10,
            ..Default::default()
        }
        .set()
        .unwrap();
        let _ = thread::Builder::new().spawn(move || loop {
            // check we got a button press
            if let Ok(_) = btn_events.recv_deadline(std::time::Instant::now()) {
                ::log::info!("got button press!")
            }
            // check buffers
            if let Ok(data) = lsm_events.recv_deadline(std::time::Instant::now()) {
                println!(
                    "{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
                    data.accel_x,
                    data.accel_y,
                    data.accel_z,
                    data.mag_x,
                    data.mag_y,
                    data.mag_z,
                    data.timestamp
                );
            }
            if let Ok(data) = bmi_events.recv_deadline(std::time::Instant::now()) {
                // println!(
                //     "{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
                //     data.accel_x,
                //     data.accel_y,
                //     data.accel_z,
                //     data.gyro_x,
                //     data.gyro_y,
                //     data.gyro_z,
                //     data.sample_time
                // );
            }
            thread::sleep(time::Duration::from_millis(50));
        });
    }
}
