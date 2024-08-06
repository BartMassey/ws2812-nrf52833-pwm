# WS2812 driver for the nRF52833 using PWM
Bart Massey 2024-08

![Maintenance](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)
[![crates-io](https://img.shields.io/crates/v/ws2812-nrf52833-pwm.svg)](https://crates.io/crates/ws2812-nrf52833-pwm)
[![api-docs](https://docs.rs/ws2812-nrf52833-pwm/badge.svg)](https://docs.rs/ws2812-nrf52833-pwm)
[![dependency-status](https://deps.rs/repo/github/BartMassey/ws2812-nrf52833-pwm/status.svg)](https://deps.rs/repo/github/BartMassey/ws2812-nrf52833-pwm)

This code is intended for usage with the
[smart-leds](https://github.com/smart-leds-rs/smart-leds)
crate.

This driver utilizes a PWM from the Nordic nRF52833 to drive
a pin on the device with the signals necessary for a
WS2812-family "Neopixel" smart LED chain.

## License

Like the work from which this was derived (see below), this
work is licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Acknowledgements

David Sawatzke <david-sawatzke@users.noreply.github.com>
wrote a driver clear back in 2017 that was the starting
point for this work. Greatly appreciated.
