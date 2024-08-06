#![doc(html_root_url = "https://docs.rs/ws2812-nrf52833-pwm/0.1.0")]
//! # Use ws2812 leds with nRF52833 PWM.
//!
//! - For usage with `smart-leds`
//! - Implements the `SmartLedsWrite` trait

#![no_std]

use core::ops::DerefMut;

use embedded_hal::delay::DelayNs;
use embedded_dma as dma;
use nrf52833_hal::{gpio, pwm};
use smart_leds_trait::{SmartLedsWrite, RGB8};

type PwmPin = gpio::Pin<gpio::Output<gpio::PushPull>>;

/// Error during WS2812 driver operation.
pub enum Error<PWM, DELAY> {
    /// PWM error.
    PwmError(pwm::Error, PWM, pwm::Pins, DELAY),
}

impl<PWM, DELAY> core::fmt::Debug for Error<PWM, DELAY> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::PwmError(err, _, _, _) => write!(f, "pwm error: {:?}", err)
        }
    }
}

/// Proxy for driving a WS2812-family device using PWM.
pub struct Ws2812<PWM, DELAY>
where
    PWM: pwm::Instance,
{
    pwm: Option<pwm::Pwm<PWM>>,
    delay: Option<DELAY>,
}

/// WS2812 0-bit high time in ns.
const T0H_NS: u32 = 400;
/// WS2812 1-bit high time in ns.
const T1H_NS: u32 = 800;
/// WS2812 total frame time in ns.
const FRAME_NS: u32 = 1250;
/// WS2812 frame reset time in µs (minimum 50µs).
const RESET_TIME: u32 = 60;

/// PWM clock in MHz.
const PWM_CLOCK: u32 = 16;

const fn to_ticks(ns: u32) -> u32 {
    ns * PWM_CLOCK / 1000
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
/// Number of PWM ticks to wait after end of bits for reset.
//const RESET_TICKS: u32 = to_ticks(RESET_TIME);

type Seq = [u16; 24];

struct DmaBuffer(Seq);

impl core::ops::Deref for DmaBuffer {
    type Target = Seq;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DmaBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

unsafe impl dma::ReadBuffer for DmaBuffer {
    type Word = u16;
    unsafe fn read_buffer(&self) -> (*const Self::Word, usize) {
        (self.0.as_ptr(), self.0.len())
    }
}

impl<PWM, DELAY> Ws2812<PWM, DELAY>
where
    PWM: pwm::Instance + core::fmt::Debug,
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
            .set_max_duty(PWM_PERIOD)
            // Set no delay between samples.
            .set_seq_refresh(pwm::Seq::Seq0, 0)
            // Set reset delay at end of sequence.
            //.set_seq_end_delay(pwm::Seq::Seq0, RESET_TICKS)
            .set_seq_end_delay(pwm::Seq::Seq0, 0)
            // Enable sample channel.
            .enable_channel(pwm::Channel::C0)
            // Enable sample group.
            .enable_group(pwm::Group::G0)
            // Play once per activation.
            .one_shot()
            // Enable but don't start.
            .enable();

        Self { pwm: Some(pwm), delay: Some(delay) }
    }

    /// Write a full grb color for ws2812 devices.
    fn write_color(&mut self, data: u32) -> Result<(), Error<PWM, DELAY>> {
        let mut buffer = DmaBuffer([0u16; 24]);
        let nbuffer = buffer.len();
        for (i, sample) in buffer.deref_mut().iter_mut().enumerate() {
            let b = (data >> (nbuffer - i - 1)) & 1;
            *sample = BITS[b as usize];
        }

        let pwm = self.pwm.take().unwrap();
        let none = <Option<DmaBuffer>>::None;
        let seq = pwm.load(
            Some(buffer),
            none,
            true,
        ).map_err(|(err, pwm, _, _)| {
            let (pwm, pin) = pwm.free();
            Error::PwmError(err, pwm, pin, self.delay.take().unwrap())
        })?;

        loop {
            if seq.is_event_triggered(pwm::PwmEvent::SeqEnd(pwm::Seq::Seq0)) {
                break;
            }
        }
        seq.stop();
        seq.reset_event(pwm::PwmEvent::LoopsDone);

        let (_, _, pwm) = seq.split();
        self.pwm = Some(pwm);

        if let Some(ref mut delay) = self.delay {
            delay.delay_us(RESET_TIME);
        } else {
            panic!();
        }

        Ok(())
    }
}

impl<PWM, DELAY> SmartLedsWrite for Ws2812<PWM, DELAY>
where
    PWM: pwm::Instance + core::fmt::Debug,
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
        for item in iterator {
            let item = item.into();
            let color = ((item.g as u32) << 16)
                | ((item.r as u32) << 8)
                | (item.b as u32);
            self.write_color(color)?;
        }
        Ok(())
    }
}
