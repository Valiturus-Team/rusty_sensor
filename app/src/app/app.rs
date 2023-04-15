use std::{
    io::{Read, Write},
    sync::{self, Arc},
    thread, time,
};

use crate::rust_proto::algorithim;
use crate::{domain::domain, rust_proto::algorithim::AlgorithimConfiguration};
use protobuf::{Message, SpecialFields};

pub struct App {
    sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
    sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
    algorithimConfiguration: algorithim::AlgorithimConfiguration,
}

impl App {
    pub fn new(
        sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
        sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
    ) -> Self {
        App {
            sensor_input_buffer,
            sensor_output_buffer,
            algorithimConfiguration: algorithim::AlgorithimConfiguration::default(),
        }
    }
    // write data to buffer
    fn write_data(self, bytes: &Vec<u8>) {
        let out_buffer = Arc::clone(&self.sensor_output_buffer);
        out_buffer.lock().unwrap().write_all(bytes).unwrap();
    }
    fn set_configuration(&mut self, conf: algorithim::AlgorithimConfiguration) {
        self.algorithimConfiguration = conf;
    }
    pub fn run(mut self) {
        let inp_buffer = Arc::clone(&self.sensor_input_buffer);
        loop {
            let mut buffer = inp_buffer.lock().unwrap(); // reads from the buffer untill an EOF is recieved
            let tmp_buffer = buffer.clone();
            buffer.clear(); // clear the bytes in the buffer once read
            println!("received bytes: {:?}", buffer);

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
                    println!("error parsing message {:?}", _err)
                }
            }
            thread::sleep(time::Duration::from_millis(50));
        }
    }
}
