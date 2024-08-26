#![allow(dead_code)]
use std::cell::RefCell;

use crate::uart::Uart;

/// Mock UART implementation for unit tests
pub struct MockUart {
    // This will hold the data that the mock UART "sends" or "receives"
    write_data: RefCell<Vec<u8>>,
    read_data: RefCell<Vec<u8>>,
}

impl MockUart {
    pub fn new() -> Self {
        MockUart {
            write_data: RefCell::new(Vec::new()),
            read_data: RefCell::new(Vec::new()),
        }
    }

    pub fn set_read_data(&self, data: Vec<u8>) {
        *self.read_data.borrow_mut() = data;
    }

    pub fn get_written_data(&self) -> Vec<u8> {
        self.write_data.borrow().clone()
    }
}

impl Uart for MockUart {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        self.write_data.borrow_mut().extend_from_slice(data);
        Ok(data.len())
    }

    fn read(&mut self) -> Option<u8> {
        if self.read_data.borrow().is_empty() {
            None
        } else {
            Some(self.read_data.borrow_mut().remove(0))
        }
    }
}
