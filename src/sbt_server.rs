#![allow(dead_code)]

use std::{collections::HashMap, thread::sleep, time::Duration, vec};

use crate::uart::{receive_multiple_packets, send_multiple_packets_with_ack, Uart};

#[repr(u8)]
pub enum SbtResponseType {
    Success = 0x00,
    InvalidRequest = 0x01,
    HandlerNotFound = 0x02,
    InternalError = 0x03,
}

type HandlerFn = Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>;

/// Simp Byte Transfer Server is a simple server that can be used to send and receive data
/// over UART. The server can be configured with a set of handlers that can be called
/// when specific commands are received.
/// Request format:
/// - first byte is command number (handler number)
/// - second byte is arguments number
/// - next is first argument length
/// - next is first argument
/// - etc.
/// Response format:
/// - first byte is response code
/// - second byte is arguments number
/// - next is first argument length
/// - next is first argument
/// - etc.
pub struct SbtServer {
    uart: Box<dyn Uart>,
    handlers: HashMap<u8, HandlerFn>,
}

impl SbtServer {
    pub fn new(uart: Box<dyn Uart>) -> Self {
        SbtServer {
            uart,
            handlers: HashMap::new(),
        }
    }

    pub fn run(&mut self, sleep_time: u64) -> Result<(), &'static str> {
        loop {
            match self.run_non_blocking() {
                Ok(()) => {
                    if sleep_time > 0 {
                        sleep(std::time::Duration::from_millis(sleep_time));
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    pub fn add_handler(&mut self, command: u8, handler: HandlerFn) {
        self.handlers.insert(command, handler);
    }

    pub fn run_non_blocking(&mut self) -> Result<(), &'static str> {
        match self.receive_request() {
            Ok(request) => {
                let response = self.process_request(request);
                self.send_response(response, 100)
            }
            Err(_) => {
                self.uart
                    .write(&[SbtResponseType::InvalidRequest as u8])
                    .unwrap();
                Ok(())
            }
        }
    }

    fn receive_request(&mut self) -> Result<Vec<u8>, &'static str> {
        receive_multiple_packets(&mut *self.uart)
    }

    fn process_request(&mut self, request: Vec<u8>) -> Vec<u8> {
        if request.is_empty() {
            return create_response(SbtResponseType::InvalidRequest, vec![]);
        }
        match self.handlers.get(&request[0]) {
            Some(handler) => {
                handler(request[0..].to_vec());
            }
            None => {
                return create_response(SbtResponseType::HandlerNotFound, vec![]);
            }
        }

        let mut response = vec![SbtResponseType::InvalidRequest as u8];
        if let Some(handler) = self.handlers.get(&request[0]) {
            response = handler(request[1..].to_vec());
        }
        response
    }
    fn send_response(&mut self, response: Vec<u8>, timeout: u64) -> Result<(), &'static str> {
        send_multiple_packets_with_ack(
            &mut *self.uart,
            &response,
            3,
            Duration::from_millis(timeout),
        )
    }
}

pub fn create_response(response_code: SbtResponseType, args: Vec<Vec<u8>>) -> Vec<u8> {
    let mut response = vec![response_code as u8];
    response.push(args.len() as u8);

    for arg in args {
        response.push(arg.len() as u8); // Length of argument
        response.extend(arg); // Argument bytes
    }

    response
}

pub fn add_argument_to_response(response: &mut Vec<u8>, arg: Vec<u8>) {
    response.push(arg.len() as u8); // Length of argument
    response.extend(arg); // Argument bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::MockUart;
    use crate::packet::Packet; // Adjust this import path as necessary

    #[test]
    fn test_receive_request_success() {
        let uart = MockUart::new();
        // Set the read data to a valid packet
        // 0x00 is the sequence number. UartServer uses receive_multiple_packets
        // and this function keeps sequence numbers
        let packet = Packet::new(vec![0x00, 0x01, 0x03]);
        uart.set_read_data(packet.to_bytes());
        let mut server = SbtServer::new(Box::new(uart));

        let request = server.receive_request().unwrap();
        assert_eq!(request, vec![0x01, 0x03]);
    }

    #[test]
    fn test_receive_request_failure() {
        let uart = MockUart::new();
        // Set the read data to a valid packet
        // 0x00 is the sequence number. UartServer uses receive_multiple_packets
        // and this function keeps sequence numbers
        let packet = Packet::new(vec![0x01, 0x01, 0x03]);
        uart.set_read_data(packet.to_bytes());
        let mut server = SbtServer::new(Box::new(uart));

        let request = server.receive_request().unwrap_err();
        assert_eq!(request, "Packet sequence out of order");
    }

    #[test]
    fn test_process_request_handler_not_found() {
        let mut server = SbtServer::new(Box::new(MockUart::new()));
        let request = vec![0x00, 0x01, 0x03];
        let response = server.process_request(request);
        assert_eq!(response, vec![SbtResponseType::HandlerNotFound as u8, 0]);
    }

    #[test]
    fn test_process_request_invalid_request() {
        let mut server = SbtServer::new(Box::new(MockUart::new()));
        let request = vec![];
        let response = server.process_request(request);
        assert_eq!(response, vec![SbtResponseType::InvalidRequest as u8, 0]);
    }

    fn test_handler_success(request: Vec<u8>) -> Vec<u8> {
        let _ = request;
        vec![SbtResponseType::Success as u8]
    }
    fn test_handler_internal_error(request: Vec<u8>) -> Vec<u8> {
        let _ = request;
        vec![SbtResponseType::InternalError as u8]
    }
    #[test]
    fn test_process_request() {
        let mut server = SbtServer::new(Box::new(MockUart::new()));
        server.add_handler(0x00, Box::new(test_handler_success));
        server.add_handler(0x01, Box::new(test_handler_internal_error));
        let request = vec![0x00, 0x03];
        let response = server.process_request(request);
        assert_eq!(response, vec![SbtResponseType::Success as u8]);
        let request = vec![0x01, 0x03];
        let response = server.process_request(request);
        assert_eq!(response, vec![SbtResponseType::InternalError as u8]);
    }

    #[test]
    fn test_create_response() {
        let response_code = SbtResponseType::Success;
        let arg1 = vec![0x01, 0x02, 0x03];
        let arg2 = vec![0x04, 0x05];
        let response = create_response(response_code, vec![arg1, arg2]);

        assert_eq!(
            response,
            vec![
                0x00, // Success response code
                0x02, // 2 arguments
                0x03, // Length of first argument
                0x01, 0x02, 0x03, // First argument
                0x02, // Length of second argument
                0x04, 0x05 // Second argument
            ]
        );
    }
}
