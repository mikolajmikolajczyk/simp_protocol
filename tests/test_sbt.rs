mod common;
use common::virtual_uart::VirtualUart;
use simp_protocol::sbt_client::SbtClient;
use simp_protocol::sbt_server::{create_response, SbtResponseType, SbtServer};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use virtual_serialport::VirtualPort;

fn test_handler_ok(_args: Vec<u8>) -> Vec<u8> {
    create_response(
        simp_protocol::sbt_server::SbtResponseType::Success,
        vec![vec![0x00, 0x01, 0x02]],
    )
}

#[test]
fn test_sbt_server_client() {
    let (mut port1, mut port2) = VirtualPort::open_pair(115200, 1024).unwrap();
    port1.set_simulate_delay(false);
    port2.set_simulate_delay(false);

    let server_running = Arc::new(RwLock::new(true));

    let srv_running_thread_1 = server_running.clone();
    let srv_running_thread_2 = server_running.clone();
    let srv_thread = thread::spawn(move || {
        let uart = VirtualUart::new(Box::new(port1));
        let mut srv = SbtServer::new(Box::new(uart), Duration::from_millis(100));
        srv.add_handler(0x00, Box::new(test_handler_ok));
        loop {
            let is_srv_running = *srv_running_thread_1.read().unwrap();
            println!("Server running: {}", is_srv_running);
            if !is_srv_running {
                break;
            }
            match srv.run_non_blocking() {
                Err(e) if e != "timed out" => {
                    // Handle the error if it's not "timed out"
                    break;
                }
                _ => {
                    // Continue looping if the result is Ok(()) or Err("timed out")
                }
            }
        }
    });

    let client_thread = thread::spawn(move || {
        let uart = VirtualUart::new(Box::new(port2));
        let mut client = SbtClient::new(Box::new(uart), Duration::from_millis(100));

        let res = client
            .send_request(0x00, vec![vec![0x00, 0x01, 0x03]])
            .unwrap();
        assert_eq!(res.response_code, SbtResponseType::Success as u8);
        assert_eq!(res.args, vec![vec![0x00, 0x01, 0x02]]);

        //This should fail server and break loop
        let res2 = client
            .send_request(0xFF, vec![vec![0x00, 0x01, 0x02]])
            .unwrap();

        assert_eq!(res2.response_code, SbtResponseType::HandlerNotFound as u8);

        let mut srv_running_val = srv_running_thread_2.write().unwrap();
        *srv_running_val = false;
    });

    // Wait for both threads to complete
    srv_thread.join().unwrap();
    client_thread.join().unwrap();
}
