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
    _delay: DELAY,
    pin: PIN,
}

#[inline(never)]
fn spin_wait(ns: u32) {
    let count = ns / (4 * 32);
    unsafe {
        core::arch::asm!(
            "2:",
            "sub {0}, #1",
            "cmp {0}, #0",
            "bne 2b",
            in(reg) count,
            options(nomem, nostack),
        );
    }
}

impl<DELAY, PIN> Ws2812<DELAY, PIN>
where
    DELAY: DelayNs,
    PIN: OutputPin,
{
    /// The delay has to have resolution of at least 3 MHz.
    pub fn new(_delay: DELAY, mut pin: PIN) -> Ws2812<DELAY, PIN> {
        pin.set_low().ok();
        Self { _delay, pin }
    }

    /// Write a single color for ws2812 devices.
    fn write_byte(&mut self, mut data: u8) {
        for _ in 0..8 {
            if (data & 0x80) == 0 {
                self.pin.set_high().ok();
                //self.delay.delay_ns(400);
                spin_wait(400);
                self.pin.set_low().ok();
                //self.delay.delay_ns(850);
                spin_wait(850);
            } else {
                self.pin.set_high().ok();
                //self.delay.delay_ns(800);
                spin_wait(800);
                self.pin.set_low().ok();
                //self.delay.delay_ns(450);
                spin_wait(450);
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
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        spin_wait(100_000);
        for item in iterator {
            let item = item.into();
            self.write_byte(item.g);
            self.write_byte(item.r);
            self.write_byte(item.b);
            spin_wait(100_000);
        }
        Ok(())
    }
}
