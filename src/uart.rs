use crate::packet::Packet;
use std::time::{Duration, Instant};

pub const ACK_BYTE: u8 = 0x06;
pub const NACK_BYTE: u8 = 0x15;
pub const STOP_SEQ_BYTE: u8 = 0x03;

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
pub fn send_packet_and_wait_ack(
    uart: &mut dyn Uart,
    packet: &Packet,
    timeout: Duration,
) -> Result<usize, &'static str> {
    let s = send_packet(uart, packet)?;
    let res = receive_packet(uart, timeout, false)?;
    if res.payload[0] == ACK_BYTE {
        Ok(s)
    } else {
        Err("No ACK received")
    }
}

/// Function to receive a packet with a timeout
pub fn receive_packet(
    uart: &mut dyn Uart,
    timeout: Duration,
    ack: bool,
) -> Result<super::packet::Packet, &'static str> {
    let start = Instant::now();
    let mut buffer = Vec::new();

    while start.elapsed() < timeout {
        if let Some(byte) = uart.read() {
            buffer.push(byte);
            if byte == super::packet::END_BYTE {
                if ack {
                    send_packet(uart, &super::packet::Packet::new(vec![ACK_BYTE]))?;
                }
                return super::packet::Packet::from_bytes(&buffer);
            }
        }
    }

    Err("Failed to receive packet: Timeout")
}

pub fn chunk_data_with_sequence(data: &[u8], max_payload_size: usize) -> Vec<Vec<u8>> {
    let mut sequence = 0u8;
    let mut packets = Vec::new();

    // Calculate chunk sizes for the first and subsequent packets
    let first_chunk_size = max_payload_size - 2; // Reserve 2 bytes for the first packet (sequence number + total packets)
    let subsequent_chunk_size = max_payload_size - 1; // Reserve 1 byte for subsequent packets (sequence number only)

    // Calculate the total number of packets
    let total_packets = if data.len() <= first_chunk_size {
        1 // If data fits in the first packet, only 1 packet is needed
    } else {
        // Otherwise, calculate the number of subsequent packets
        ((data.len() - first_chunk_size + subsequent_chunk_size - 1) / subsequent_chunk_size + 1)
            as u8
    };

    // Handle the first packet (sequence number + total packets)
    let first_chunk = &data[..first_chunk_size.min(data.len())];
    let mut first_packet = vec![sequence, total_packets];
    first_packet.extend_from_slice(first_chunk);
    packets.push(first_packet);

    // Handle subsequent packets if there is more data
    let mut data_index = first_chunk_size;
    sequence = sequence.wrapping_add(1);

    while data_index < data.len() {
        let chunk_end = (data_index + subsequent_chunk_size).min(data.len());
        let chunk = &data[data_index..chunk_end];
        let mut packet_payload = vec![sequence]; // Add sequence number
        packet_payload.extend_from_slice(chunk); // Add chunk data
        packets.push(packet_payload);

        data_index = chunk_end;
        sequence = sequence.wrapping_add(1); // Increment sequence number, wrapping on overflow
    }

    packets
}

pub fn send_data_in_sequence(
    uart: &mut dyn Uart,
    data: &[u8],
    timeout: Duration,
    ack: bool,
) -> Result<usize, &'static str> {
    let max_payload_size = 250; // Maximum payload size per packet
    let mut total_bytes_sent = 0; // Track the total bytes sent

    // Use the helper function to get all the packets with sequence numbers
    let packets = chunk_data_with_sequence(data, max_payload_size);

    for packet_payload in packets {
        // Create a new packet with the sequence number and payload
        let packet = Packet::new(packet_payload);

        // Send the packet, with or without waiting for an ACK based on `ack` flag
        if ack {
            match send_packet_and_wait_ack(uart, &packet, timeout) {
                Ok(bytes_sent) => total_bytes_sent += bytes_sent,
                Err(e) => return Err(e), // Return error if sending fails
            }
        } else {
            match send_packet(uart, &packet) {
                Ok(bytes_sent) => total_bytes_sent += bytes_sent,
                Err(e) => return Err(e), // Return error if sending fails
            }
        }
    }

    Ok(total_bytes_sent)
}

pub fn receive_data_in_sequence(
    uart: &mut dyn Uart,
    timeout: Duration,
    ack_required: bool,
) -> Result<Vec<u8>, &'static str> {
    let mut total_data = Vec::new(); // Store all received data
    let mut expected_sequence = 0u8; // Track the expected sequence number
    let mut total_packets: Option<u8> = None; // Will store the total number of packets

    while total_packets.is_none() || expected_sequence < total_packets.unwrap() {
        // Receive a packet
        let packet = match receive_packet(uart, timeout, false) {
            Ok(packet) => packet,
            Err(e) => return Err(e), // Return error if receiving fails
        };

        // Check if the packet is properly formed
        if packet.payload.is_empty() {
            return Err("Invalid packet: no payload");
        }

        // Extract the sequence number (first byte of the payload)
        let sequence_number = packet.payload[0];

        // Verify that the sequence number matches the expected one
        if sequence_number != expected_sequence {
            return Err("Packet out of sequence");
        }

        // Handle the first packet to extract the total packet count
        if expected_sequence == 0 {
            if packet.payload.len() < 2 {
                return Err("Invalid first packet: missing total packet count");
            }
            total_packets = Some(packet.payload[1]);
            total_data.extend_from_slice(&packet.payload[2..]); // Append the rest of the payload
        } else {
            // Append the rest of the payload (excluding the sequence number)
            total_data.extend_from_slice(&packet.payload[1..]);
        }

        // If ACK is required, send it back to confirm receipt of the packet
        if ack_required {
            let ack_packet = Packet::new(vec![ACK_BYTE]);
            match send_packet(uart, &ack_packet) {
                Ok(_) => {}              // Successfully sent ACK
                Err(e) => return Err(e), // Return error if ACK sending fails
            }
        }

        // Increment the expected sequence number
        expected_sequence = expected_sequence.wrapping_add(1);
    }

    Ok(total_data) // Return the collected data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mocks::MockUart;
    use crate::packet::{self, Packet}; // Adjust this import path as necessary

    #[test]
    fn test_send_packet() {
        let mut uart = MockUart::new();
        let packet = Packet::new(vec![0x01, 0x02, 0x03]);
        let res = send_packet(&mut uart, &packet);
        assert!(res.is_ok());
        assert_eq!(uart.get_written_data(), packet.to_bytes());
    }

    #[test]
    fn test_send_packet_and_wait_ack() {
        let mut uart = MockUart::new();
        let packet = Packet::new(vec![0x01, 0x02, 0x03]);
        uart.set_read_data(Packet::new(vec![ACK_BYTE]).to_bytes());
        let res = send_packet_and_wait_ack(&mut uart, &packet, Duration::from_millis(100));
        assert!(res.is_ok());
    }

    #[test]
    fn test_receive_packet() {
        let mut uart = MockUart::new();
        let packet = Packet::new(vec![0x01, 0x02, 0x03]);
        uart.set_read_data(packet.to_bytes());
        let res = receive_packet(&mut uart, Duration::from_millis(100), true);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), packet);
    }

    #[test]
    fn test_send_data_in_sequence() {
        let mut uart = MockUart::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let res = send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 11);
    }

    #[test]
    fn test_send_data_in_sequence_with_ack() {
        let mut uart = MockUart::new();
        uart.set_read_data(Packet::new(vec![ACK_BYTE]).to_bytes());
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let res = send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), true);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 11);
    }

    #[test]
    fn test_send_data_in_sequence_multiple_packets() {
        let mut uart = MockUart::new();

        // Generate 500 bytes of data to send
        let data: Vec<u8> = (0x01..=0xFA).collect(); // Generates 250 bytes of data

        // Call send_data_in_sequence to send the data
        let res = send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), false);

        // Check if the function succeeded
        assert!(res.is_ok());

        let expected_packets = chunk_data_with_sequence(&data, 250);
        // Convert all expected packets into one vec
        let mut expected_data = Vec::new();
        for packet in expected_packets {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        assert_eq!(res.unwrap(), expected_data.len());
        assert_eq!(uart.get_write_count(), 2);
        assert_eq!(uart.get_written_data(), expected_data);
    }

    #[test]
    fn test_send_data_in_sequence_multiple_packets_with_ack() {
        let mut uart = MockUart::new();

        // Generate 500 bytes of data to send
        let data: Vec<u8> = (0x01..=0xFA).collect(); // Generates 250 bytes of data

        let mut acks = Vec::new();
        acks.extend(Packet::new(vec![ACK_BYTE]).to_bytes());
        acks.extend(Packet::new(vec![ACK_BYTE]).to_bytes());
        uart.set_read_data(acks);
        // Call send_data_in_sequence to send the data
        let res = send_data_in_sequence(&mut uart, &data, Duration::from_millis(100), true);

        // Check if the function succeeded
        assert!(res.is_ok());

        let expected_packets = chunk_data_with_sequence(&data, 250);
        // Convert all expected packets into one vec
        let mut expected_data = Vec::new();
        for packet in expected_packets {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        assert_eq!(res.unwrap(), expected_data.len());
        assert_eq!(uart.get_write_count(), 2);
        assert_eq!(uart.get_written_data(), expected_data);
    }

    #[test]
    fn test_receive_data_in_sequence_simple() {
        let mut uart = MockUart::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let chunks = chunk_data_with_sequence(&data, 5);
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);
        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);
    }

    #[test]
    fn test_receive_data_in_sequence_multiple_packets() {
        let mut uart = MockUart::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let chunks = chunk_data_with_sequence(&data, 5);
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);

        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);
    }

    #[test]
    fn test_receive_data_in_sequence_small_data() {
        let mut uart = MockUart::new();
        let data = vec![0x01]; // Very small data
        let chunks = chunk_data_with_sequence(&data, 5);
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);

        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);
    }

    #[test]
    fn test_receive_data_in_sequence_exact_fit() {
        let mut uart = MockUart::new();
        let data = vec![0x01, 0x02, 0x03]; // Data that fits exactly in a packet
        let chunks = chunk_data_with_sequence(&data, 5); // Adjusted payload size
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);

        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);
    }

    #[test]
    fn test_receive_data_in_sequence_large_data() {
        let mut uart = MockUart::new();
        let data = (1..=50).collect::<Vec<u8>>(); // Large data set
        let chunks = chunk_data_with_sequence(&data, 10); // 10 bytes max payload size
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);

        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), false);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);
    }

    #[test]
    fn test_receive_data_in_sequence_with_ack() {
        let mut uart = MockUart::new();
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let chunks = chunk_data_with_sequence(&data, 5);
        let mut expected_data = Vec::new();
        for packet in chunks {
            expected_data.extend(Packet::new(packet).to_bytes()); // Append each packet's data to the result
        }
        uart.set_read_data(expected_data);

        let res = receive_data_in_sequence(&mut uart, Duration::from_millis(100), true); // ACK required
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), data);

        // Ensure that ACKs were sent after each packet
        let sent_data = uart.get_written_data();
        let mut main_vector = Vec::new();
        let sub_vectors = vec![
            packet::Packet::new(vec![ACK_BYTE]).to_bytes(),
            packet::Packet::new(vec![ACK_BYTE]).to_bytes(),
        ];

        // Extend the main vector with each sub-vector
        for sub_vector in sub_vectors {
            main_vector.extend(sub_vector);
        }
        assert_eq!(sent_data, main_vector);
    }

    #[test]
    fn test_chunk_data_with_sequence_normal_case() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
        let max_payload_size = 7; // 5 bytes for data + 1 byte for the sequence number
        let packets = chunk_data_with_sequence(&data, max_payload_size);
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0], vec![0x00, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05]);
    }

    #[test]
    fn test_chunk_data_with_sequence_overflow_case() {
        let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]; // 6 bytes of data
        let max_payload_size = 6; // 4 bytes of data in first packet, and remaining in the second

        let packets = chunk_data_with_sequence(&data, max_payload_size);

        // 1st packet will contain the first 4 bytes of data (since 2 bytes are reserved),
        // 2nd packet will contain the remaining 2 bytes of data
        assert_eq!(packets.len(), 2);

        // First packet should contain 4 bytes of data (sequence number 0, total packets 2)
        assert_eq!(packets[0], vec![0x00, 0x02, 0x01, 0x02, 0x03, 0x04]);

        // Second packet should contain the remaining 2 bytes of data
        assert_eq!(packets[1], vec![0x01, 0x05, 0x06]);
    }

    #[test]
    fn test_chunk_data_with_sequence_small_data() {
        let data = vec![0x01, 0x02];
        let max_payload_size = 4; // 3 bytes max payload, so data should fit comfortably with sequence + total_packets

        let packets = chunk_data_with_sequence(&data, max_payload_size);

        // Only one packet is expected, as all data fits with the sequence and total_packets in the first packet
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0], vec![0x00, 0x01, 0x01, 0x02]);
    }
}
