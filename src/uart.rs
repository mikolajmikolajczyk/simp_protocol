use crate::packet::Packet;
use std::time::{Duration, Instant};

pub const ACK_BYTE: u8 = 0x06;
pub const NACK_BYTE: u8 = 0x15;

/// Trait for UART communication
///
/// This trait needs to be implemented in order for the library to work.
/// All functions depend on the implementation of this trait.
pub trait Uart {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str>;
    fn read(&mut self) -> Option<u8>;
}

/// Function to send a packet without waiting for an ACK
pub fn send_packet(uart: &mut dyn Uart, packet: &Packet) -> Result<usize, &'static str> {
    uart.write(&packet.to_bytes())
        .map_err(|_| "Failed to send packet")
}

/// Function to send a packet and wait for an ACK
pub fn send_packet_with_ack(
    uart: &mut dyn Uart,
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

/// Function to receive a packet
pub fn receive_packet(uart: &mut dyn Uart) -> Result<super::packet::Packet, &'static str> {
    let mut buffer = Vec::new();
    while let Some(byte) = uart.read() {
        buffer.push(byte);
        if byte == super::packet::END_BYTE {
            return super::packet::Packet::from_bytes(&buffer);
        }
    }
    Err("Failed to receive packet")
}

/// Function to send multiple packets
pub fn send_multiple_packets_with_ack(
    uart: &mut dyn Uart,
    data: &Vec<u8>,
    retries: usize,
    timeout: Duration,
) -> Result<(), &'static str> {
    let max_payload_size = 250; // Max size for the payload part of the packet
    let mut sequence = 0u8;

    for chunk in data.chunks(max_payload_size) {
        // Each chunk gets a sequence byte, which counts toward the payload size limit
        let mut packet_data = vec![sequence];
        packet_data.extend_from_slice(chunk);
        let packet = Packet::new(packet_data);

        // Send packet and expect an ACK
        send_packet_with_ack(uart, &packet, retries, timeout)?;

        // Increment sequence number, wrapping on overflow
        sequence = sequence.wrapping_add(1);
    }

    Ok(())
}

/// Function to receive multiple packets
pub fn receive_multiple_packets(uart: &mut dyn Uart) -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    let mut expected_sequence = 0u8;

    loop {
        let packet = receive_packet(uart)?;
        if packet.payload.is_empty() {
            return Err("Empty packet received");
        }

        let sequence = packet.payload[0];
        if sequence != expected_sequence {
            return Err("Packet sequence out of order");
        }

        data.extend_from_slice(&packet.payload[1..]);
        expected_sequence = expected_sequence.wrapping_add(1);

        if packet.payload.len() < 250 {
            // If the last packet's payload is less than max, it is the final packet
            break;
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::MockUart;
    use std::cell::RefCell;



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

    #[test]
    fn test_send_multiple_packets_with_ack() {
        let mut uart = MockUart::new();
        let data = vec![0x02; 800]; // Data larger than 256 bytes

        // Simulate an ACK for each packet sent
        uart.set_read_data(vec![ACK_BYTE; 4]);

        let result =
            send_multiple_packets_with_ack(&mut uart, &data, 3, Duration::from_millis(500));
        assert!(result.is_ok());

        // Extract sent data for debugging
        let sent_data = uart.get_written_data();
        println!("Sent Data: {:?}", sent_data);

        // Define the expected number of packets (800 bytes, max 250 payload per packet, so 4 packets)
        let max_payload_size = 250;
        let mut expected_sequence = 0u8;

        // Iterate over chunks of sent data, assuming each packet is prefixed with START_BYTE and ends with END_BYTE
        let mut offset = 0;
        while offset < sent_data.len() {
            assert_eq!(sent_data[offset], crate::packet::START_BYTE); // Check start byte
            offset += 1;

            let length = sent_data[offset] as usize; // Get the packet length
            offset += 1;

            assert_eq!(sent_data[offset], expected_sequence); // Check sequence number
            offset += 1;

            // Calculate expected payload length
            let payload_length = length - 1; // Length includes sequence byte but not checksum

            // Verify payload bytes
            let payload_end = offset + payload_length;
            assert!(payload_end < sent_data.len());

            let payload = &sent_data[offset..payload_end];
            let expected_payload_start = (expected_sequence as usize) * (max_payload_size - 1);
            let expected_payload_end = expected_payload_start + payload.len();
            let expected_payload = &data[expected_payload_start..expected_payload_end];
            assert_eq!(payload, expected_payload);

            offset = payload_end;

            // Verify checksum
            let checksum_start = offset - payload_length - 1; // sequence byte + payload
            let checksum_data = &sent_data[checksum_start..payload_end];
            let calculated_checksum = Packet::calculate_checksum(checksum_data);
            let actual_checksum = sent_data[offset];
            assert_eq!(actual_checksum, calculated_checksum);
            offset += 1;

            assert_eq!(sent_data[offset], crate::packet::END_BYTE); // Check end byte
            offset += 1;

            // Increment sequence number, wrapping on overflow
            expected_sequence = expected_sequence.wrapping_add(1);
        }

        // Ensure we processed the correct number of packets
        assert_eq!(expected_sequence, 4); // Should have sent 4 packets
    }

    #[test]
    fn test_receive_multiple_packets() {
        let mut uart = MockUart::new();
        let data = vec![0x01; 600]; // Data larger than 256 bytes

        // Create packets with sequence numbers and set to mock UART
        let mut packet_data = Vec::new();
        let mut sequence = 0u8;
        for chunk in data.chunks(250) {
            let mut chunk_with_seq = vec![sequence];
            chunk_with_seq.extend_from_slice(chunk);
            let packet = Packet::new(chunk_with_seq);
            packet_data.extend(packet.to_bytes());
            sequence = sequence.wrapping_add(1);
        }
        uart.set_read_data(packet_data);

        let result = receive_multiple_packets(&mut uart);
        assert!(result.is_ok());

        let received_data = result.unwrap();
        assert_eq!(received_data, data);
    }
}
