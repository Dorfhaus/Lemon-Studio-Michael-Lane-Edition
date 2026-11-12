// Copyright (C) 2026 ItsAlphaNeon
// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of Lemon Studio: https://github.com/ItsAlphaNeon/Lemon-Studio

//! Minimal WAV writer (32-bit float, stereo).

use std::io::{self, Write};

/// Writes interleaved stereo `f32` samples as a 32-bit IEEE float WAV file.
pub fn write_f32_stereo(w: &mut impl Write, rate: u32, interleaved: &[f32]) -> io::Result<()> {
    write_f32_stereo_header(w, rate, (interleaved.len() / 2) as u64)?;
    for s in interleaved {
        w.write_all(&s.to_le_bytes())?;
    }
    Ok(())
}

/// The header of a 32-bit IEEE float stereo WAV file of `frames` frames at `rate`; the samples follow it.
pub fn write_f32_stereo_header(w: &mut impl Write, rate: u32, frames: u64) -> io::Result<()> {
    write_stereo_header(w, rate, frames, 32)
}

/// Stereo recording header: 16/24-bit PCM or 32-bit IEEE float.
pub fn write_stereo_header(w: &mut impl Write, rate: u32, frames: u64, bits: u16) -> io::Result<()> {
    const CHANNELS: u16 = 2;
    let format: u16 = match bits {
        16 | 24 => 1,
        32 => 3,
        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "unsupported WAV bit depth")),
    };
    let block_align = CHANNELS * bits / 8;
    let data_len = u32::try_from(frames.saturating_mul(u64::from(block_align)))
        .ok()
        .filter(|&n| n <= u32::MAX - 36)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "too long for WAV"))?;
    w.write_all(b"RIFF")?;
    w.write_all(&(36 + data_len).to_le_bytes())?;
    w.write_all(b"WAVEfmt ")?;
    w.write_all(&16u32.to_le_bytes())?;
    w.write_all(&format.to_le_bytes())?;
    w.write_all(&CHANNELS.to_le_bytes())?;
    w.write_all(&rate.to_le_bytes())?;
    w.write_all(&(rate * u32::from(block_align)).to_le_bytes())?;
    w.write_all(&block_align.to_le_bytes())?;
    w.write_all(&bits.to_le_bytes())?;
    w.write_all(b"data")?;
    w.write_all(&data_len.to_le_bytes())
}
