#![allow(dead_code)]
use std::time::Duration;

use crate::uart::{receive_multiple_packets, send_multiple_packets_with_ack, Uart};

#[derive(Debug, PartialEq)]
pub struct SbtResponse {
    response_code: u8,
    args: Vec<Vec<u8>>,
}

impl SbtResponse {
    pub fn new(response_code: u8, args: Vec<Vec<u8>>) -> Self {
        SbtResponse {
            response_code,
            args,
        }
    }
}

pub struct SbtClient {
    uart: Box<dyn Uart>,
}

impl SbtClient {
    pub fn new(uart: Box<dyn Uart>) -> Self {
        SbtClient { uart }
    }

    pub fn send_request(
        &mut self,
        command: u8,
        args: Vec<Vec<u8>>,
    ) -> Result<SbtResponse, &'static str> {
        let mut request = vec![command];
        for arg in args {
            request.push(arg.len() as u8);
            request.extend(arg);
        }
        match send_multiple_packets_with_ack(
            &mut *self.uart,
            &request,
            3,
            Duration::from_millis(100),
        ) {
            Ok(()) => self.receive_response(),
            Err(err) => Err(err),
        }
    }

    fn receive_response(&mut self) -> Result<SbtResponse, &'static str> {
        match receive_multiple_packets(&mut *self.uart) {
            Ok(response) => {
                let response_code = response[0];
                let arg_count = response[1];

                let mut args: Vec<Vec<u8>> = Vec::new();

                let mut response_index = 2;
                for i in 0..arg_count {
                    let arg_len = response[response_index] as usize;
                    response_index += 1;
                    let arg = response[response_index..response_index + arg_len].to_vec();
                    response_index += arg_len;
                    args.push(arg);
                }
                Ok(SbtResponse::new(response_code, args))
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mocks::MockUart, uart::ACK_BYTE};

    #[test]
    fn test_send_request() {
        let uart = Box::new(MockUart::new());
        let packet =
            crate::packet::Packet::new(vec![0x00, 0x01, 0x01, 0x03, 0x01, 0x02, 0x03]).to_bytes();

        uart.set_read_data(vec![ACK_BYTE].into_iter().chain(packet).collect());
        let mut client = SbtClient::new(uart);
        let response = client
            .send_request(0x01, vec![vec![0x01, 0x02, 0x03]])
            .unwrap();

        assert_eq!(
            response,
            SbtResponse::new(0x01, vec![vec![0x01, 0x02, 0x03]])
        );
    }

    #[test]
    fn test_send_request_multiple_arguments() {
        let uart = Box::new(MockUart::new());
        let packet = crate::packet::Packet::new(vec![
            0x00, 0x01, 0x02, 0x03, 0x01, 0x02, 0x03, 0x05, 0x01, 0x02, 0x04, 0x03, 0x05,
        ])
        .to_bytes();

        uart.set_read_data(vec![ACK_BYTE].into_iter().chain(packet).collect());
        let mut client = SbtClient::new(uart);
        let response = client
            .send_request(0x01, vec![vec![0x01, 0x02, 0x03]])
            .unwrap();

        assert_eq!(
            response,
            SbtResponse::new(
                0x01,
                vec![vec![0x01, 0x02, 0x03], vec![0x01, 0x02, 0x04, 0x03, 0x05]]
            )
        );
    }
}
