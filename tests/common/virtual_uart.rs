use virtual_serialport::VirtualPort;

use simp_protocol::uart::Uart;
use std::io::{Read, Write};

/// Mock UART implementation for unit tests
pub struct VirtualUart {
    port: Box<VirtualPort>,
}

impl VirtualUart {
    pub fn new(port: Box<VirtualPort>) -> Self {
        VirtualUart { port }
    }
}

impl Uart for VirtualUart {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        match self.port.write_all(data) {
            Ok(_) => Ok(data.len()),
            Err(err) => Err(Box::leak(err.to_string().into_boxed_str())),
        }
    }

    fn read(&mut self) -> Option<u8> {
        let buf = &mut [0u8; 1];
        match self.port.read_exact(buf) {
            Ok(_) => Some(buf[0]),
            Err(_) => None,
        }
    }
}
