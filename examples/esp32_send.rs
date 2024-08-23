/*
This code needs to be compiled using esp32 target!!!
This won't compile on regular PCs!!!
Please read [this](https://docs.esp-rs.org/book/) book for more information.
*/
use esp_idf_svc::hal::delay;
use esp_idf_svc::hal::gpio;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::uart::*;

use simp_protocol::packet::Packet;
use simp_protocol::uart::send_packet;

pub struct ESPUart<'a> {
    uart_driver: UartDriver<'a>,
}

impl<'a> ESPUart<'a> {
    pub fn new(uart_driver: UartDriver<'a>) -> Self {
        Self { uart_driver }
    }
}

impl<'a> simp_protocol::uart::Uart for ESPUart<'a> {
    fn write(&mut self, data: &[u8]) -> Result<usize, &'static str> {
        self.uart_driver
            .write(data)
            .map_err(|_| "Failed to write data")
    }

    fn read(&mut self) -> Option<u8> {
        let mut buffer = [0u8; 1];
        match self.uart_driver.read(&mut buffer, 100) {
            Ok(1) => Some(buffer[0]),
            _ => None,
        }
    }
}

fn main() {
    esp_idf_svc::sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let tx = peripherals.pins.gpio0;
    let rx = peripherals.pins.gpio3;

    let config = config::Config::new().baudrate(Hertz(115_200));
    let uart = UartDriver::new(
        peripherals.uart0,
        tx,
        rx,
        Option::<gpio::Gpio0>::None,
        Option::<gpio::Gpio1>::None,
        &config,
    )
    .unwrap();

    let mut esp_uart = ESPUart::new(uart);
    let pkg = Packet::new(b"HELLO WORLD".to_vec());

    loop {
        send_packet(&mut esp_uart, &pkg).unwrap();
        delay::Ets::delay_ms(1000_u32);
    }
}
