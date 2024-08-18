#![no_main]
#![no_std]

use smart_leds::RGB8;
use smart_leds_trait::SmartLedsWrite;
use ws2812_nrf52833_pwm::Ws2812;

use cortex_m_rt::entry;
use embedded_hal::delay::DelayNs;
use microbit::{board::Board, hal::Timer};
use panic_rtt_target as _;
use rtt_target::rtt_init_print;
#[cfg(feature = "tick")]
use rtt_target::{rprint, rprintln};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut timer = Timer::new(board.TIMER0);
    let pin = board.edge.e16.degrade();
    let mut ws2812: Ws2812<{ 4 * 24 }, _> = Ws2812::new(board.PWM0, pin);

    let leds = [
        RGB8::new(255, 0, 0),
        RGB8::new(0, 255, 0),
        RGB8::new(0, 0, 255),
        RGB8::new(255, 255, 0),
        RGB8::new(0, 255, 255),
        RGB8::new(255, 0, 255),
        RGB8::new(20, 20, 20),
        RGB8::new(0, 0, 0),
    ];

    #[cfg(feature = "tick")]
    rprintln!("starting");

    ws2812.write(leds[..4].iter().cloned()).unwrap();

    #[cfg(feature = "tick")]
    rprintln!("displaying indices");

    timer.delay_ms(3000);

    #[cfg(feature = "tick")]
    rprintln!("starting loop");

    let nleds = leds.len();
    let mut start = 0;
    loop {
        let mut cur_leds: [RGB8; 4] = Default::default();
        for i in 0..4 {
            cur_leds[i] = leds[(i + start) % nleds];
        }
        let tmp = cur_leds[0];
        cur_leds[0] = RGB8::new(255, 255, 255);
        ws2812.write(cur_leds).unwrap();
        #[cfg(feature = "tick")]
        rprint!("tick");
        cur_leds[0] = tmp;
        ws2812.write(cur_leds).unwrap();
        #[cfg(feature = "tick")]
        rprintln!(".");
        timer.delay_ms(500);
        start = (start + 1) % nleds;
    }
}
