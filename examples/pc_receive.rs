use clap::Parser;
use simp_protocol::uart::receive_packet;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "ez80fd")]
#[command(about="Client cli for ez80fd", long_about = None)]
struct Cli {
    #[arg(short, long, default_value_t = 115200)]
    baudrate: u32,
    #[arg(short, long, default_value_t = String::from("COM6"))]
    port: String,
}

pub struct PCUart {
    serial_port: Box<dyn serialport::SerialPort>,
}

impl<'a> PCUart {
    pub fn new(baudrate: u32, port: &'a str) -> Self {
        let serial_port = serialport::new(port, baudrate)
            .open()
            .expect("Failed to open serial port");
        Self { serial_port }
    }
}

impl<'a> simp_protocol::uart::Uart for PCUart {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        self.serial_port
            .write(data)
            .map_err(|_| "Failed to write to serial port")
    }

    fn read(&mut self) -> Option<u8> {
        let mut buffer = [0u8; 1];
        match self.serial_port.read(&mut buffer) {
            Ok(1) => Some(buffer[0]),
            _ => None,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let mut pc_uart = PCUart::new(cli.baudrate, cli.port.as_str());

    println!("Waiting for chip info...");

    loop {
        match receive_packet(&mut pc_uart) {
            Ok(packet) => {
                // Convert the packet payload (Vec<u8>) to a String
                match String::from_utf8(packet.payload.to_vec()) {
                    Ok(string) => println!("Packet received: {}", string),
                    Err(e) => eprintln!("Failed to convert packet to string: {}", e),
                }
                break; // Exit the loop upon successful reception and conversion
            }
            Err(e) => {
                //eprintln!("Failed to receive packet, retrying... Error: {}", e);
                sleep(Duration::from_millis(100)); // Wait briefly before retrying
            }
        }
    }
}
