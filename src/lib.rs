//! # Use ws2812 leds with embedded-hal 1.0 Delay.
//!
//! - For usage with `smart-leds`
//! - Implements the `SmartLedsWrite` trait
//!
//! The Delay given to `new()` must run at an absolute
//! minimum of 3 MHz to work.
//!
//! If it's too slow (e.g.  e.g. all/some leds are white or
//! display the wrong color) you may want to try the `slow`
//! feature.

#![no_std]

use embedded_hal::{digital::OutputPin, delay::DelayNs};
use smart_leds_trait::{SmartLedsWrite, RGB8};

pub struct Ws2812<DELAY, PIN> {
    delay: DELAY,
    pin: PIN,
}

impl<DELAY, PIN> Ws2812<DELAY, PIN>
where
    DELAY: DelayNs,
    PIN: OutputPin,
{
    /// The delay has to have resolution of at least 3 MHz.
    pub fn new(delay: DELAY, mut pin: PIN) -> Ws2812<DELAY, PIN> {
        pin.set_low().ok();
        Self { delay, pin }
    }

    /// Write a single color for ws2812 devices.
    fn write_byte(&mut self, mut data: u8) {
        for _ in 0..8 {
            if (data & 0x80) != 0 {
                self.pin.set_high().ok();
                self.delay.delay_ns(800);
                self.pin.set_low().ok();
                self.delay.delay_ns(450);
            } else {
                self.pin.set_high().ok();
                self.delay.delay_ns(400);
                self.pin.set_low().ok();
                self.delay.delay_ns(850);
            }
            data <<= 1;
        }
    }
}

impl<DELAY, PIN> SmartLedsWrite for Ws2812<DELAY, PIN>
where
    DELAY: DelayNs,
    PIN: OutputPin,
{
    type Error = ();
    type Color = RGB8;
    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: Iterator<Item = I>,
        I: Into<Self::Color>,
    {
        for item in iterator {
            // minimum 50 µs for reset.
            self.delay.delay_us(50);
            let item = item.into();
            self.write_byte(item.g);
            self.write_byte(item.r);
            self.write_byte(item.b);
        }
        Ok(())
    }
}
