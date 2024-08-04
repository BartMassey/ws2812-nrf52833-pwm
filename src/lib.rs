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

use core::ops::DerefMut;

use cortex_m::{delay::Delay, peripheral::{SYST, syst::SystClkSource}};
use embedded_dma as dma;
use nrf52833_hal::{gpio, pwm};
use smart_leds_trait::{SmartLedsWrite, RGB8};

type PwmPin = gpio::Pin<gpio::Output<gpio::PushPull>>;

pub struct Ws2812<PWM: pwm::Instance + core::fmt::Debug> {
    pwm: Option<pwm::Pwm<PWM>>,
    delay: Delay,
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

#[derive(Debug)]
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
    

impl<PWM> Ws2812<PWM>
where
    PWM: pwm::Instance + core::fmt::Debug,
{
    pub fn new(pwm: PWM, syst: SYST, pin: PwmPin) -> Self {
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

        let delay = Delay::with_source(syst, 64_000_000, SystClkSource::Core);

        Self { pwm: Some(pwm), delay }
    }

    /// Write a full grb color for ws2812 devices.
    fn write_color(&mut self, data: u32) {
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
        ).unwrap();

        loop {
            if seq.is_event_triggered(pwm::PwmEvent::SeqEnd(pwm::Seq::Seq0)) {
                break;
            }
        }
        seq.stop();
        seq.reset_event(pwm::PwmEvent::LoopsDone);

        let (_, _, pwm) = seq.split();
        self.pwm = Some(pwm);

        self.delay.delay_us(RESET_TIME);
    }
}

impl<PWM> SmartLedsWrite for Ws2812<PWM>
where
    PWM: pwm::Instance + core::fmt::Debug,
{
    type Error = ();
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
            self.write_color(color);
        }
        Ok(())
    }
}
