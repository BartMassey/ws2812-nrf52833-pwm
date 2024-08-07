#![no_main]
#![no_std]

use smart_leds::RGB8;
use smart_leds_trait::SmartLedsWrite;
use ws2812_nrf52833_pwm::Ws2812;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{
    board::Board,
    hal::{
        gpio::{DriveConfig, Level},
        Timer,
    },
};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let pin = board
        .edge
        .e16
        .into_push_pull_output_drive(Level::Low, DriveConfig::HighDrive0HighDrive1);
    let delay = Timer::new(board.TIMER1);
    let mut ws2812 = Ws2812::new(board.PWM0, delay, pin.degrade());

    let leds = [
        RGB8::new(255u8, 0, 0),
        RGB8::new(0, 255u8, 0),
        RGB8::new(0, 0, 255u8),
        RGB8::new(255u8, 255u8, 255u8),
        RGB8::new(255u8, 255u8, 0),
        RGB8::new(0, 255u8, 255u8),
        RGB8::new(255u8, 0, 255u8),
    ];
    let nleds = leds.len();
    let mut start = 0;
    loop {
        let mut cur_leds: [RGB8; 4] = Default::default();
        for i in 0..4 {
            cur_leds[i] = leds[(i + start) % nleds];
        }
        ws2812.write(cur_leds).unwrap();
        timer.delay_ms(500);
        start = (start + 1) % nleds;
    }
}
