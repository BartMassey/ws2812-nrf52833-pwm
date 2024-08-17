#![doc(html_root_url = "https://docs.rs/ws2812-nrf52833-pwm/0.1.0")]
//! # Use ws2812 leds with nRF52833 PWM.
//!
//! - For usage with `smart-leds`
//! - Implements the `SmartLedsWrite` trait

#![no_std]

use core::ops::DerefMut;

use embedded_dma as dma;
use embedded_hal::delay::DelayNs;
use nrf52833_hal::{gpio, pwm};
use smart_leds_trait::{SmartLedsWrite, RGB8};

pub type PwmPin = gpio::Pin<gpio::Output<gpio::PushPull>>;

/// Error during WS2812 driver operation.
pub enum Error<PWM, DELAY> {
    /// PWM error.
    PwmError(pwm::Error, PWM, pwm::Pins, DELAY),
}

impl<PWM, DELAY> core::fmt::Debug for Error<PWM, DELAY> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PwmError(err, _, _, _) => write!(f, "pwm error: {:?}", err),
        }
    }
}

/// Proxy for driving a chain of `N` WS2812-family device using PWM.
pub struct Ws2812<const N: usize, PWM, DELAY>
where
    PWM: pwm::Instance,
{
    pwm: Option<pwm::Pwm<PWM>>,
    delay: Option<DELAY>,
    buf: Option<DmaBuffer<N>>,
}

/// WS2812 0-bit high time in ns.
const T0H_NS: u32 = 400;
/// WS2812 1-bit high time in ns.
const T1H_NS: u32 = 800;
/// WS2812 total frame time in ns.
const FRAME_NS: u32 = 1250;
/// WS2812 frame reset time in µs (minimum 250µs for some BC).
const RESET_TIME: u32 = 300;

/// PWM clock in MHz.
const PWM_CLOCK: u32 = 16;

const fn to_ticks(ns: u32) -> u32 {
    (ns * PWM_CLOCK + 500) / 1000
}

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

impl<const N: usize, PWM, DELAY> Ws2812<N, PWM, DELAY>
where
    PWM: pwm::Instance,
    DELAY: DelayNs,
{
    /// Set up for WS2812 bit transfers.
    pub fn new(pwm: PWM, delay: DELAY, pin: PwmPin) -> Self {
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
            // Be sure to be advancing the thing.
            .set_step_mode(pwm::StepMode::Auto)
            // Set maximum duty cycle = PWM period in ticks.
            .set_max_duty(PWM_PERIOD);

        Self {
            pwm: Some(pwm),
            buf: Some(DmaBuffer::default()),
            delay: Some(delay),
        }
    }
}

impl<const N: usize, PWM, DELAY> SmartLedsWrite for Ws2812<N, PWM, DELAY>
where
    PWM: pwm::Instance,
    DELAY: DelayNs,
{
    type Error = Error<PWM, DELAY>;
    type Color = RGB8;
    /// Write all the items of an iterator to a ws2812 strip
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color>,
    {
        let mut delay = self.delay.take().unwrap();
        delay.delay_us(RESET_TIME);
        self.delay = Some(delay);

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
            // Set no delay between samples.
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            // Set reset delay at end of sequence.
            //.set_seq_end_delay(pwm::Seq::Seq0, RESET_TICKS)
            .set_seq_end_delay(pwm::Seq::Seq0, 0)
            // Enable sample channel.
            .enable_channel(pwm::Channel::C0)
            // Enable sample group.
            .enable_group(pwm::Group::G0)
            // Run this waveform once.
            .one_shot()
            // Enable now.
            .enable();
        let none = <Option<DmaBuffer<N>>>::None;
        let seq = pwm
            .load(Some(buffer), none, false)
            .map_err(|(err, pwm, _, _)| {
                let (pwm, pin) = pwm.free();
                Error::PwmError(err, pwm, pin, self.delay.take().unwrap())
            })?;

        let end_event = pwm::PwmEvent::SeqEnd(pwm::Seq::Seq0);
        seq.reset_event(end_event);
        seq.start_seq(pwm::Seq::Seq0);
        loop {
            if seq.is_event_triggered(end_event) {
                seq.stop();
                break;
            }
        }

        let (buffer, _, pwm) = seq.split();
        pwm.stop();
        self.pwm = Some(pwm);
        self.buf = buffer;

        Ok(())
    }
}
