#![no_main]
#![no_std]

use ws2812_hal_delay::Ws2812;
use smart_leds::RGB8;
use smart_leds_trait::SmartLedsWrite;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal::{gpio::{Level, DriveConfig}, Timer}};
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let pin = board.edge.e16.into_push_pull_output_drive(
        Level::Low,
        DriveConfig::HighDrive0HighDrive1,
    );
    let mut ws2812 = Ws2812::new(board.PWM0, board.SYST, pin.degrade());
    
    let leds = [
        RGB8::new(255u8, 0, 0),
        RGB8::new(0, 255u8, 0),
        RGB8::new(0, 0, 255u8),
        RGB8::new(255u8, 255u8, 255u8),
    ];
    loop {
        rprintln!("start");
        ws2812.write(leds.into_iter()).unwrap();
        rprintln!("done");
        timer.delay_ms(1000);
    }
}
