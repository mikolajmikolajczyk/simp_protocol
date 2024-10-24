#![allow(dead_code)]
use std::time::Duration;

use crate::uart::{receive_data_in_sequence, send_data_in_sequence, Uart};

#[derive(Debug, PartialEq)]
pub struct SbtResponse {
    pub response_code: u8,
    pub args: Vec<Vec<u8>>,
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
    timeout: Duration,
}

impl SbtClient {
    pub fn new(uart: Box<dyn Uart>, timeout: Duration) -> Self {
        SbtClient { uart, timeout }
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
        match send_data_in_sequence(&mut *self.uart, &request, self.timeout, true) {
            Ok(_) => self.receive_response(),
            Err(err) => Err(err),
        }
    }

    fn receive_response(&mut self) -> Result<SbtResponse, &'static str> {
        match receive_data_in_sequence(&mut *self.uart, self.timeout, true) {
            Ok(response) => {
                let response_code = response[0];
                let arg_count = response[1];

                let mut args: Vec<Vec<u8>> = Vec::new();

                let mut response_index = 2;
                for _ in 0..arg_count {
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
