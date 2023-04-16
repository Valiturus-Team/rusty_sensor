use std::{
    io::{Read, Write},
    sync::{self, Arc},
    thread, time,
};

use crate::{domain::domain, rust_proto::algorithim::AlgorithimConfiguration};
use crate::{domain::domain::ButtonEvent, rust_proto::algorithim};
use crossbeam_channel::{unbounded, Receiver, Sender};
use protobuf::{Message, SpecialFields};

pub struct App {
    sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
    sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
    algorithim_configuration: algorithim::AlgorithimConfiguration,
    buttonEvents: Receiver<domain::ButtonEvent>,
}

impl App {
    pub fn new(
        sensor_input_buffer: Arc<sync::Mutex<Vec<u8>>>,
        sensor_output_buffer: Arc<sync::Mutex<Vec<u8>>>,
        button_eventer: Receiver<ButtonEvent>,
    ) -> Self {
        App {
            sensor_input_buffer,
            sensor_output_buffer,
            algorithim_configuration: algorithim::AlgorithimConfiguration::default(),
            buttonEvents: button_eventer,
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
        let btn_events = self.buttonEvents.clone();
        thread::spawn(move || loop {
            match btn_events.recv() {
                Ok(_) => {
                    println!("got button press!")
                }
                Err(err) => println!("error receiving event {:?}", err),
            }
        });

        let inp_buffer = Arc::clone(&self.sensor_input_buffer);
        loop {
            let mut buffer = inp_buffer.lock().unwrap();
            let tmp_buffer = buffer.clone();
            buffer.clear();
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

// todo
trait ReadBMI {}

// todo
trait ReadLSM {}

pub trait ButtonEvents {
    fn button_channel(&self) -> Receiver<domain::ButtonEvent>; // gets the button channel
}
