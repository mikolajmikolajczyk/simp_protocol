use crate::packet::Packet;
use std::time::{Duration, Instant};

const ACK_BYTE: u8 = 0x06;
const NACK_BYTE: u8 = 0x15;

pub trait Uart {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str>;
    fn read(&mut self) -> Option<u8>;
}

// Function to send a packet without waiting for an ACK
pub fn send_packet(uart: &mut impl Uart, packet: &Packet) -> Result<usize, &'static str> {
    uart.write(&packet.to_bytes())
        .map_err(|_| "Failed to send packet")
}

// Function to send a packet and wait for an ACK
pub fn send_packet_with_ack(
    uart: &mut impl Uart,
    packet: &Packet,
    retries: usize,
    timeout: Duration,
) -> Result<(), &'static str> {
    for _ in 0..retries {
        // Send the packet without waiting for ACK
        send_packet(uart, packet)?;

        // Wait for ACK or NACK
        let start_time = Instant::now();
        while start_time.elapsed() < timeout {
            if let Some(response) = uart.read() {
                if response == ACK_BYTE {
                    // ACK received, success
                    return Ok(());
                } else if response == NACK_BYTE {
                    // NACK received, retry sending
                    break;
                }
            }
        }
        // Timeout, retry sending
    }
    Err("Failed to send packet after retries")
}
pub fn receive_packet(uart: &mut impl Uart) -> Result<super::packet::Packet, &'static str> {
    let mut buffer = Vec::new();
    while let Some(byte) = uart.read() {
        buffer.push(byte);
        if byte == super::packet::END_BYTE {
            return super::packet::Packet::from_bytes(&buffer);
        }
    }
    Err("Failed to receive packet")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockUart {
        // This will hold the data that the mock UART "sends" or "receives"
        write_data: RefCell<Vec<u8>>,
        read_data: RefCell<Vec<u8>>,
    }

    impl MockUart {
        fn new() -> Self {
            MockUart {
                write_data: RefCell::new(Vec::new()),
                read_data: RefCell::new(Vec::new()),
            }
        }

        fn set_read_data(&self, data: Vec<u8>) {
            *self.read_data.borrow_mut() = data;
        }

        fn get_written_data(&self) -> Vec<u8> {
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::RefCell;
    
        struct MockUart {
            // This will hold the data that the mock UART "sends" or "receives"
            write_data: RefCell<Vec<u8>>,
            read_data: RefCell<Vec<u8>>,
        }
    
        impl MockUart {
            fn new() -> Self {
                MockUart {
                    write_data: RefCell::new(Vec::new()),
                    read_data: RefCell::new(Vec::new()),
                }
            }
    
            fn set_read_data(&self, data: Vec<u8>) {
                *self.read_data.borrow_mut() = data;
            }
    
            fn get_written_data(&self) -> Vec<u8> {
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
    
        #[test]
        fn test_send_packet() {
            let mut uart = MockUart::new();
            let packet = Packet::new(vec![0x01, 0x02, 0x03]);
    
            let result = send_packet(&mut uart, &packet);
            assert!(result.is_ok());
    
            // Verify that the correct data was "sent"
            let expected_data = packet.to_bytes();
            assert_eq!(uart.get_written_data(), expected_data);
        }
    
        #[test]
        fn test_send_packet_with_ack_success() {
            let mut uart = MockUart::new();
            let packet = Packet::new(vec![0x01, 0x02, 0x03]);
    
            // Set the mock to return an ACK after the packet is sent
            uart.set_read_data(vec![ACK_BYTE]);
    
            let result = send_packet_with_ack(&mut uart, &packet, 3, Duration::from_millis(500));
            assert!(result.is_ok());
    
            // Verify that the correct data was "sent"
            let expected_data = packet.to_bytes();
            assert_eq!(uart.get_written_data(), expected_data);
        }
    
        #[test]
        fn test_send_packet_with_ack_failure() {
            let mut uart = MockUart::new();
            let packet = Packet::new(vec![0x01, 0x02, 0x03]);
    
            // Set the mock to return nothing (no ACK or NACK)
            uart.set_read_data(vec![]);
    
            let result = send_packet_with_ack(&mut uart, &packet, 3, Duration::from_millis(500));
            assert!(result.is_err());
            assert_eq!(result.err().unwrap(), "Failed to send packet after retries");
    
            // Verify that the packet was sent 3 times due to retries
            let expected_data = packet.to_bytes();
            let expected_sent_data = expected_data.repeat(3);
            assert_eq!(uart.get_written_data(), expected_sent_data);
        }
    
        #[test]
        fn test_receive_packet_success() {
            let mut uart = MockUart::new();
            let packet = Packet::new(vec![0x01, 0x02, 0x03]);
    
            // Set the mock to provide the bytes of a complete packet
            uart.set_read_data(packet.to_bytes());
    
            let result = receive_packet(&mut uart);
            assert!(result.is_ok());
    
            // Verify the received packet is as expected
            let received_packet = result.unwrap();
            assert_eq!(received_packet.payload, packet.payload);
        }
    
        #[test]
        fn test_receive_packet_failure() {
            let mut uart = MockUart::new();
    
            // Set the mock to provide an incomplete packet
            uart.set_read_data(vec![crate::packet::START_BYTE, 0x03, 0x01, 0x02]);
    
            let result = receive_packet(&mut uart);
            assert!(result.is_err());
            assert_eq!(result.err().unwrap(), "Failed to receive packet");
        }
    }
}