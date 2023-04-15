use std::io::{Write, Read};



pub trait ReadWrite: Write + Read {}

// blanket implementation:
impl<T: Write + Read> ReadWrite for T {}