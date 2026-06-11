//! Authentic Kaypro keyboard bell (Ctrl-G / ASCII 7).
//!
//! On a real Kaypro the beeper lives in the *detachable keyboard*, not the
//! main unit. The keyboard's Mitsubishi 8049 microcontroller (6 MHz) drives a
//! small piezo speaker from pin P2.5, generating a **1.5625 kHz square wave**
//! using its internal timer/counter. The host requests a beep by sending a
//! command byte to the keyboard over the SIO Channel B serial link (our port
//! 0x05); the keyboard supports a short beep (the Ctrl-G bell) and a long beep.
//!
//! Reference: MAME `src/devices/bus/keytronic/kay_kbd.cpp`:
//!   "P2.5 drives the speaker (1.5625kHz tone generated using timer/counter)."
//!
//! We reproduce the tone exactly with a continuously-running output stream
//! (via `cpal`, behind the optional `audio` feature) that emits silence until
//! `Beeper::beep_short`/`beep_long` arms a countdown of square-wave samples.
//! Builds without the `audio` feature (e.g. the static-musl terminal release)
//! never construct a `Beeper`, and the machine falls back to a terminal BEL.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Cloneable trigger shared between the emulator and the audio callback.
///
/// Always compiled so `KayproMachine` can hold an `Option<Beeper>` regardless
/// of whether the `audio` feature is enabled; it is only ever *constructed* by
/// the `audio`-gated [`AudioEngine`].
#[derive(Clone)]
#[cfg_attr(not(feature = "audio"), allow(dead_code))]
pub struct Beeper {
    /// Square-wave samples still to emit; the audio callback decrements this.
    remaining: Arc<AtomicU32>,
    short_samples: u32,
    long_samples: u32,
}

impl Beeper {
    /// Arm the short beep — the ASCII-7 (Ctrl-G) bell.
    pub fn beep_short(&self) {
        self.remaining.store(self.short_samples, Ordering::Relaxed);
    }

    /// Arm the long beep.
    pub fn beep_long(&self) {
        self.remaining.store(self.long_samples, Ordering::Relaxed);
    }
}

#[cfg(feature = "audio")]
pub use backend::AudioEngine;

#[cfg(feature = "audio")]
mod backend {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// Exact Kaypro keyboard beep frequency (Hz) — 8049 P2.5 square wave.
    const KAYPRO_BELL_HZ: f32 = 1562.5;
    /// Short-beep (Ctrl-G bell) duration. The exact 8049 firmware timing is
    /// undocumented; ~100 ms matches the perceived length of the real bell.
    const SHORT_BEEP_MS: u32 = 100;
    /// Long-beep duration.
    const LONG_BEEP_MS: u32 = 250;
    /// Output level of the square wave (the real piezo is modest, not full scale).
    const AMPLITUDE: f32 = 0.18;

    /// Owns the live `cpal` output stream. Must be kept alive for as long as
    /// the bell should be audible (i.e. for the whole interactive run).
    pub struct AudioEngine {
        _stream: cpal::Stream,
        beeper: Beeper,
    }

    impl AudioEngine {
        /// Open the default output device and start a silent, always-running
        /// stream. Returns `None` (rather than failing) if there is no audio
        /// device or the platform refuses the stream — the caller then falls
        /// back to a terminal BEL.
        pub fn new() -> Option<AudioEngine> {
            let host = cpal::default_host();
            let device = host.default_output_device()?;
            let supported = device.default_output_config().ok()?;
            let sample_rate = supported.sample_rate().0 as f32;
            let sample_format = supported.sample_format();
            let config: cpal::StreamConfig = supported.into();

            let remaining = Arc::new(AtomicU32::new(0));
            // Samples per full square-wave cycle at the device's sample rate.
            let period = sample_rate / KAYPRO_BELL_HZ;

            let stream = match sample_format {
                cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config, remaining.clone(), period),
                cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config, remaining.clone(), period),
                cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config, remaining.clone(), period),
                _ => return None,
            }
            .ok()?;
            stream.play().ok()?;

            let beeper = Beeper {
                remaining,
                short_samples: ((sample_rate * SHORT_BEEP_MS as f32) / 1000.0) as u32,
                long_samples: ((sample_rate * LONG_BEEP_MS as f32) / 1000.0) as u32,
            };
            Some(AudioEngine { _stream: stream, beeper })
        }

        /// A cloneable trigger handle to hand to the machine.
        pub fn beeper(&self) -> Beeper {
            self.beeper.clone()
        }
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        remaining: Arc<AtomicU32>,
        period: f32,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        let half = period * 0.5;
        let mut phase = 0.0f32;
        device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let value: f32 = if remaining.load(Ordering::Relaxed) > 0 {
                        remaining.fetch_sub(1, Ordering::Relaxed);
                        if phase < half { AMPLITUDE } else { -AMPLITUDE }
                    } else {
                        0.0
                    };
                    phase += 1.0;
                    if phase >= period {
                        phase -= period;
                    }
                    let sample = T::from_sample(value);
                    for out in frame.iter_mut() {
                        *out = sample;
                    }
                }
            },
            |err| eprintln!("Kaypro bell audio stream error: {}", err),
            None,
        )
    }
}
