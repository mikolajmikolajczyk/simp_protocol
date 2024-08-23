pub const START_BYTE: u8 = 0x7E;
pub const END_BYTE: u8 = 0x7F;
pub const ESCAPE_BYTE: u8 = 0x7D;
pub const ESCAPE_XOR: u8 = 0x20;

pub struct Packet {
    pub start_byte: u8,
    pub length: u8,
    pub payload: Vec<u8>,
    pub checksum: u8,
    pub end_byte: u8,
}

impl Packet {
    pub fn new(payload: Vec<u8>) -> Self {
        let escaped_payload = Self::escape_payload(&payload);
        let length = escaped_payload.len() as u8;
        let checksum = Self::calculate_checksum(&escaped_payload);
        Packet {
            start_byte: START_BYTE,
            length,
            payload: escaped_payload,
            checksum,
            end_byte: END_BYTE,
        }
    }

    pub fn calculate_checksum(payload: &[u8]) -> u8 {
        payload.iter().fold(0u8, |acc, &x| acc.wrapping_add(x))
    }

    pub fn escape_payload(payload: &[u8]) -> Vec<u8> {
        let mut escaped_payload = Vec::new();
        for &byte in payload {
            match byte {
                START_BYTE | END_BYTE | ESCAPE_BYTE => {
                    escaped_payload.push(ESCAPE_BYTE);
                    escaped_payload.push(byte ^ ESCAPE_XOR);
                }
                _ => escaped_payload.push(byte),
            }
        }
        escaped_payload
    }

    pub fn unescape_payload(payload: &[u8]) -> Vec<u8> {
        let mut unescaped_payload = Vec::new();
        let mut escape_next = false;

        for &byte in payload {
            if escape_next {
                unescaped_payload.push(byte ^ ESCAPE_XOR);
                escape_next = false;
            } else if byte == ESCAPE_BYTE {
                escape_next = true;
            } else {
                unescaped_payload.push(byte);
            }
        }
        unescaped_payload
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.start_byte, self.length];
        bytes.extend(&self.payload);
        bytes.push(self.checksum);
        bytes.push(self.end_byte);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 4 || bytes[0] != START_BYTE || bytes[bytes.len() - 1] != END_BYTE {
            return Err("Invalid packet structure");
        }
        let length = bytes[1] as usize;
        let checksum = bytes[bytes.len() - 2];
        let payload = &bytes[2..bytes.len() - 2];
        let unescaped_payload = Self::unescape_payload(payload);

        if checksum != Self::calculate_checksum(&unescaped_payload) {
            return Err("Checksum mismatch");
        }

        Ok(Packet {
            start_byte: START_BYTE,
            length: length as u8,
            payload: unescaped_payload,
            checksum,
            end_byte: END_BYTE,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_creation() {
        let payload = vec![0x01, 0x02, 0x03];
        let packet = Packet::new(payload.clone());

        assert_eq!(packet.start_byte, START_BYTE);
        assert_eq!(packet.end_byte, END_BYTE);
        assert_eq!(packet.length, packet.payload.len() as u8);
        assert_eq!(packet.checksum, Packet::calculate_checksum(&packet.payload));
        assert_eq!(packet.payload, Packet::escape_payload(&payload));
    }

    #[test]
    fn test_checksum_calculation() {
        let payload = vec![0x01, 0x02, 0x03];
        let checksum = Packet::calculate_checksum(&payload);
        assert_eq!(checksum, 0x01 + 0x02 + 0x03);
    }

    #[test]
    fn test_escaping_payload() {
        let payload = vec![START_BYTE, 0x01, END_BYTE, ESCAPE_BYTE, 0x02];
        let escaped_payload = Packet::escape_payload(&payload);
        let expected = vec![
            ESCAPE_BYTE, START_BYTE ^ ESCAPE_XOR, 
            0x01, 
            ESCAPE_BYTE, END_BYTE ^ ESCAPE_XOR, 
            ESCAPE_BYTE, ESCAPE_BYTE ^ ESCAPE_XOR, 
            0x02
        ];
        assert_eq!(escaped_payload, expected);
    }

    #[test]
    fn test_unescaping_payload() {
        let escaped_payload = vec![
            ESCAPE_BYTE, START_BYTE ^ ESCAPE_XOR, 
            0x01, 
            ESCAPE_BYTE, END_BYTE ^ ESCAPE_XOR, 
            ESCAPE_BYTE, ESCAPE_BYTE ^ ESCAPE_XOR, 
            0x02
        ];
        let unescaped_payload = Packet::unescape_payload(&escaped_payload);
        let expected = vec![START_BYTE, 0x01, END_BYTE, ESCAPE_BYTE, 0x02];
        assert_eq!(unescaped_payload, expected);
    }

    #[test]
    fn test_to_bytes() {
        let payload = vec![0x01, 0x02, 0x03];
        let packet = Packet::new(payload.clone());
        let bytes = packet.to_bytes();

        let mut expected = vec![START_BYTE, packet.length];
        expected.extend_from_slice(&Packet::escape_payload(&payload));
        expected.push(packet.checksum);
        expected.push(END_BYTE);

        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_from_bytes() {
        let payload = vec![0x01, 0x02, 0x03];
        let packet = Packet::new(payload.clone());
        let bytes = packet.to_bytes();

        let parsed_packet = Packet::from_bytes(&bytes).expect("Failed to parse packet");
        assert_eq!(parsed_packet.start_byte, START_BYTE);
        assert_eq!(parsed_packet.end_byte, END_BYTE);
        assert_eq!(parsed_packet.length, packet.length);
        assert_eq!(parsed_packet.checksum, packet.checksum);
        assert_eq!(parsed_packet.payload, payload);
    }

    #[test]
    fn test_from_bytes_with_invalid_checksum() {
        let payload = vec![0x01, 0x02, 0x03];
        let packet = Packet::new(payload.clone());
        let mut bytes = packet.to_bytes();
    
        // Store the index of the checksum to avoid borrowing issues
        let checksum_index = bytes.len() - 2;
    
        // Corrupt the checksum
        bytes[checksum_index] = packet.checksum.wrapping_add(1);
    
        let result = Packet::from_bytes(&bytes);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Checksum mismatch");
    }

    #[test]
    fn test_from_bytes_with_invalid_structure() {
        let invalid_bytes = vec![0x00, 0x01, 0x02]; // No START_BYTE, no END_BYTE
        let result = Packet::from_bytes(&invalid_bytes);
        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), "Invalid packet structure");
    }
}
