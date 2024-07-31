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
    let count = ns / (5 * 32);
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

    /// Write a full grb color for ws2812 devices.
    fn write_color(&mut self, mut data: u32) {
        for _ in 0..24 {
            if (data & 0x800000) == 0 {
                self.pin.set_high().ok();
                spin_wait(400);
                self.pin.set_low().ok();
                spin_wait(850);
            } else {
                self.pin.set_high().ok();
                spin_wait(800);
                self.pin.set_low().ok();
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
            let color = ((item.g as u32) << 16)
                | ((item.r as u32) << 8)
                | (item.b as u32);
            self.write_color(color);
            spin_wait(100_000);
        }
        Ok(())
    }
}
