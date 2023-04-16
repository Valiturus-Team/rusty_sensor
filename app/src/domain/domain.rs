use std::io::{Read, Write};

pub trait ReadWrite: Write + Read {}

// blanket implementation:
impl<T: Write + Read> ReadWrite for T {}

// pub struct ButtonEvent(bool);

pub type ButtonEvent = bool;
