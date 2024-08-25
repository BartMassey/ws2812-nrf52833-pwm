#![doc(html_root_url = "https://docs.rs/ws2812-nrf52833-pwm/0.2.1")]
/*! # Use ws2812 leds with nRF52833 PWM.

This code drives a WS2812-family LED chain (should work with
WS2812, WS2812B/C) using a PWM unit of the nRF52833. The PWM
unit makes it easy to get the precise fast timing needed for
these chips.

This crate is intended for usage with the `smart-leds`
crate: it implements the `SmartLedsWrite` trait.

*/

#![no_std]

use core::ops::DerefMut;

use embedded_dma as dma;
use nrf52833_hal::{gpio, pwm};
use smart_leds_trait::{SmartLedsWrite, RGB8};

/// Error during WS2812 driver operation.
pub enum Error<PWM> {
    /// PWM error.
    PwmError(pwm::Error, PWM, pwm::Pins),
}

impl<PWM> core::fmt::Debug for Error<PWM> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PwmError(err, _, _) => write!(f, "pwm error: {:?}", err),
        }
    }
}

/// Driver for a chain of WS2812-family devices using
/// PWM. The constant `N` should be 24 times the number of
/// chips in the chain.
pub struct Ws2812<const N: usize, PWM>
where
    PWM: pwm::Instance,
{
    pwm: Option<pwm::Pwm<PWM>>,
    buf: Option<DmaBuffer<N>>,
}

/// WS2812 0-bit high time in ns.
const T0H_NS: u32 = 400;
/// WS2812 1-bit high time in ns.
const T1H_NS: u32 = 800;
/// WS2812 total frame time in ns.
const FRAME_NS: u32 = 1250;
/// WS2812 frame reset time in µs (minimum 250µs for some BC, plus slop).
const RESET_TIME: u32 = 270;

/// PWM clock in MHz.
const PWM_CLOCK: u32 = 16;

/// Convert nanoseconds to PWM ticks, rounding.
const fn to_ticks(ns: u32) -> u32 {
    (ns * PWM_CLOCK + 500) / 1000
}

/// WS2812 frame reset time in PWM ticks.
const RESET_TICKS: u32 = to_ticks(RESET_TIME * 1000);

/// Samples for PWM array, with flip bits.
const BITS: [u16; 2] = [
    // 0-bit high time in ticks.
    to_ticks(T0H_NS) as u16 | 0x8000,
    // 1-bit high time in ticks.
    to_ticks(T1H_NS) as u16 | 0x8000,
];
/// Total PWM period in ticks.
const PWM_PERIOD: u16 = to_ticks(FRAME_NS) as u16;

type Seq<const N: usize> = [u16; N];

struct DmaBuffer<const N: usize>(Seq<N>);

impl<const N: usize> Default for DmaBuffer<N> {
    fn default() -> Self {
        DmaBuffer([0; N])
    }
}

impl<const N: usize> core::ops::Deref for DmaBuffer<N> {
    type Target = Seq<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> DerefMut for DmaBuffer<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl<const N: usize> dma::ReadBuffer for DmaBuffer<N> {
    type Word = u16;
    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.0.as_ptr(), self.0.len())
    }
}

impl<const N: usize, PWM> Ws2812<N, PWM>
where
    PWM: pwm::Instance,
{
    /// Set up WS2812 chain with PWM and an output pin.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let board = microbit::Board::take().unwrap();
    /// let ws2812: Ws2812<{4 * 24}, _, _> = Ws2812::new(board.PWM0, board.edge.e16.degrade());
    /// ```
    pub fn new<PinMode>(pwm: PWM, pin: gpio::Pin<PinMode>) -> Self {
        // Use high drive to get faster rise/fall times. Probably unnecessary.
        let pin = pin
            .into_push_pull_output_drive(gpio::Level::Low, gpio::DriveConfig::HighDrive0HighDrive1);
        let pwm = pwm::Pwm::new(pwm);
        pwm
            // output the waveform on the speaker pin
            .set_output_pin(pwm::Channel::C0, pin)
            // Prescaler set for 16MHz.
            .set_prescaler(pwm::Prescaler::Div1)
            // Configure for up counter mode.
            .set_counter_mode(pwm::CounterMode::Up)
            // Read duty cycle values from sequence.
            .set_load_mode(pwm::LoadMode::Common)
            // Set maximum duty cycle = PWM period in ticks.
            .set_max_duty(PWM_PERIOD);

        Self {
            pwm: Some(pwm),
            buf: Some(DmaBuffer::default()),
        }
    }
}

impl<const N: usize, PWM> SmartLedsWrite for Ws2812<N, PWM>
where
    PWM: pwm::Instance,
{
    type Error = Error<PWM>;
    type Color = RGB8;
    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let mut buffer = self.buf.take().unwrap();

        for (item, locs) in iterator.into_iter().zip(buffer.chunks_mut(24)) {
            let item = item.into();
            let color = ((item.g as u32) << 16) | ((item.r as u32) << 8) | (item.b as u32);
            for (i, loc) in locs.iter_mut().enumerate() {
                let b = (color >> (24 - i - 1)) & 1;
                *loc = BITS[b as usize];
            }
        }

        let pwm = self.pwm.take().unwrap();
        pwm
            // Be sure to be advancing the thing.
            .set_step_mode(pwm::StepMode::Auto)
            // Set no delay between samples.
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            // Set reset delay at end of sequence 0.
            .set_seq_end_delay(pwm::Seq::Seq0, RESET_TICKS)
            // Set no delay between samples.
            .set_seq_refresh(pwm::Seq::Seq1, 0)
            // Set no delay at end of sequence 1.
            .set_seq_end_delay(pwm::Seq::Seq1, 0)
            // Enable sample channel.
            .enable_channel(pwm::Channel::C0)
            // Enable sample group.
            .enable_group(pwm::Group::G0)
            // Run this waveform once.
            .repeat(1)
            // Enable now.
            .enable();
        let pre = DmaBuffer([0x8000]);
        let seq = pwm
            .load(Some(pre), Some(buffer), false)
            .map_err(|(err, pwm, _, _)| {
                let (pwm, pin) = pwm.free();
                Error::PwmError(err, pwm, pin)
            })?;

        let end_event = pwm::PwmEvent::LoopsDone;
        seq.reset_event(end_event);
        seq.start_seq(pwm::Seq::Seq0);
        loop {
            if seq.is_event_triggered(end_event) {
                seq.stop();
                break;
            }
        }

        let (_, buffer, pwm) = seq.split();
        pwm.stop();
        self.pwm = Some(pwm);
        self.buf = buffer;

        Ok(())
    }
}
