mod common;
use common::virtual_uart::VirtualUart;
use simp_protocol::{
    packet::*,
    uart::{
        receive_data_in_sequence, receive_packet, send_data_in_sequence, send_packet,
        send_packet_and_wait_ack,
    },
};
use std::{thread, time::Duration};
use virtual_serialport::VirtualPort;

#[test]
fn test_send_receive_packet() {
    let (mut port1, mut port2) = VirtualPort::open_pair(115200, 1024).unwrap();
    port1.set_simulate_delay(false);
    port2.set_simulate_delay(false);

    let send_thread = thread::spawn(move || {
        let mut uart = VirtualUart::new(Box::new(port1));
        let packet = Packet::new(vec![0x01, 0x02, 0x03]);
        let size = send_packet(&mut uart, &packet).unwrap();
        assert_eq!(size, packet.to_bytes().len());
    });

    let recv_thread = thread::spawn(move || {
        let mut uart = VirtualUart::new(Box::new(port2));
        let received = receive_packet(&mut uart, Duration::from_millis(100), false).unwrap();
        assert_eq!(received, Packet::new(vec![0x01, 0x02, 0x03]));
    });

    send_thread.join().unwrap();
    recv_thread.join().unwrap();
}

#[test]
fn test_send_receive_packet_with_ack() {
    let (mut port1, mut port2) = VirtualPort::open_pair(115200, 1024).unwrap();
    port1.set_simulate_delay(false);
    port2.set_simulate_delay(false);

    let send_thread = thread::spawn(move || {
        let mut uart = VirtualUart::new(Box::new(port1));
        let packet = Packet::new(vec![0x01, 0x02, 0x03]);
        let size =
            send_packet_and_wait_ack(&mut uart, &packet, Duration::from_millis(100)).unwrap();
        assert_eq!(size, packet.to_bytes().len());
    });

    let recv_thread = thread::spawn(move || {
        let mut uart = VirtualUart::new(Box::new(port2));
        let received = receive_packet(&mut uart, Duration::from_millis(100), true).unwrap();
        assert_eq!(received, Packet::new(vec![0x01, 0x02, 0x03]));
    });

    send_thread.join().unwrap();
    recv_thread.join().unwrap();
}

#[test]
fn test_send_receive_multiple_packets() {
    let (mut port1, mut port2) = VirtualPort::open_pair(115200, 1024).unwrap();
    port1.set_simulate_delay(false);
    port2.set_simulate_delay(false);

    let send_thread = thread::spawn(move || {
        let data = vec![0x01; 600]; // Data larger than 256 bytes
        let mut uart = VirtualUart::new(Box::new(port1));
        send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), false).unwrap();
    });

    let receive_thread = thread::spawn(move || {
        let data = vec![0x01; 600]; // Data larger than 256 bytes
        let mut uart = VirtualUart::new(Box::new(port2));
        loop {
            if let Ok(received_data) =
                receive_data_in_sequence(&mut uart, Duration::from_millis(100), false)
            {
                assert_eq!(received_data, data);
                break;
            }
        }
    });

    // Wait for both threads to complete
    send_thread.join().unwrap();
    receive_thread.join().unwrap();
}

#[test]
fn test_send_receive_multiple_packets_with_ack() {
    let (mut port1, mut port2) = VirtualPort::open_pair(115200, 1024).unwrap();
    port1.set_simulate_delay(false);
    port2.set_simulate_delay(false);

    let send_thread = thread::spawn(move || {
        let data = vec![0x01; 600]; // Data larger than 256 bytes
        let mut uart = VirtualUart::new(Box::new(port1));
        send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), true).unwrap();
    });

    let receive_thread = thread::spawn(move || {
        let data = vec![0x01; 600]; // Data larger than 256 bytes
        let mut uart = VirtualUart::new(Box::new(port2));
        loop {
            if let Ok(received_data) =
                receive_data_in_sequence(&mut uart, Duration::from_millis(100), true)
            {
                assert_eq!(received_data, data);
                break;
            }
        }
    });

    // Wait for both threads to complete
    send_thread.join().unwrap();
    receive_thread.join().unwrap();
}
