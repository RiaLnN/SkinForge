use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use skinforge_core::categorize::ComponentSlot;
use std::io::Cursor;
use std::path::Path;
use std::sync::Mutex;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    active_sink: Mutex<Option<(Sink, Option<ComponentSlot>)>>,
    volume: Mutex<f32>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn new() -> Self {
        match OutputStream::try_default() {
            Ok((stream, handle)) => Self {
                _stream: Some(stream),
                stream_handle: Some(handle),
                active_sink: Mutex::new(None),
                volume: Mutex::new(0.7),
            },
            Err(err) => {
                eprintln!("Failed to initialize audio output stream: {err}");
                Self {
                    _stream: None,
                    stream_handle: None,
                    active_sink: Mutex::new(None),
                    volume: Mutex::new(0.7),
                }
            }
        }
    }

    pub fn volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    pub fn set_volume(&self, vol: f32) {
        let clamped = vol.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
        if let Ok(mut guard) = self.active_sink.lock() {
            if let Some((sink, _)) = guard.as_mut() {
                sink.set_volume(clamped);
            }
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.active_sink.lock() {
            if let Some((sink, _)) = guard.take() {
                sink.stop();
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Ok(mut guard) = self.active_sink.lock() {
            if let Some((sink, _)) = guard.as_ref() {
                if !sink.empty() {
                    return true;
                }
            }
            *guard = None;
        }
        false
    }

    pub fn is_playing_slot(&self, slot: ComponentSlot) -> bool {
        if let Ok(mut guard) = self.active_sink.lock() {
            if let Some((sink, active_slot)) = guard.as_ref() {
                if !sink.empty() {
                    return *active_slot == Some(slot);
                }
            }
            *guard = None;
        }
        false
    }

    /// Toggles playback for a slot: if `slot` is currently playing, stops it;
    /// otherwise stops any previous sound and plays the slot's file (or built-in default fallback).
    pub fn toggle_slot(&self, slot: ComponentSlot, file_path: Option<&Path>) {
        if self.is_playing_slot(slot) {
            self.stop();
            return;
        }
        self.play_slot(slot, file_path);
    }

    /// Plays the slot's file if available and valid, or falls back to the built-in default osu! sound.
    pub fn play_slot(&self, slot: ComponentSlot, file_path: Option<&Path>) {
        if let Some(path) = file_path {
            if let Ok(bytes) = std::fs::read(path) {
                if self.play_bytes_with_slot(bytes, Some(slot)) {
                    return;
                }
            }
        }
        let fallback_bytes = synthesize_default_wav(slot);
        let _ = self.play_bytes_with_slot(fallback_bytes, Some(slot));
    }

    pub fn play_from_file(&self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            self.play_bytes(bytes);
        }
    }

    pub fn play_bytes(&self, bytes: Vec<u8>) {
        let _ = self.play_bytes_with_slot(bytes, None);
    }

    fn play_bytes_with_slot(&self, bytes: Vec<u8>, slot: Option<ComponentSlot>) -> bool {
        let Some(handle) = &self.stream_handle else {
            return false;
        };

        let cursor = Cursor::new(bytes);
        let Ok(source) = Decoder::new(cursor) else {
            return false;
        };

        let Ok(sink) = Sink::try_new(handle) else {
            return false;
        };

        let vol = self.volume();
        sink.set_volume(vol);
        sink.append(source);

        if let Ok(mut guard) = self.active_sink.lock() {
            if let Some((old_sink, _)) = guard.take() {
                old_sink.stop();
            }
            *guard = Some((sink, slot));
        }

        true
    }
}

/// Synthesizes a clean 16-bit 44.1kHz mono PCM WAV fallback sound for a given `ComponentSlot`
/// so skins that rely on osu!'s built-in default sounds can still be previewed.
pub fn synthesize_default_wav(slot: ComponentSlot) -> Vec<u8> {
    let sample_rate: u32 = 44_100;
    let (duration_ms, base_freq, end_freq, noise_mix): (u32, f32, f32, f32) = match slot {
        ComponentSlot::Hitnormal => (65, 520.0, 180.0, 0.15),
        ComponentSlot::Hitwhistle => (140, 1280.0, 1560.0, 0.02),
        ComponentSlot::Hitfinish => (180, 440.0, 220.0, 0.55),
        ComponentSlot::Hitclap => (85, 880.0, 340.0, 0.65),
        ComponentSlot::Combobreak => (260, 320.0, 110.0, 0.35),
        ComponentSlot::SectionPass => (320, 523.25, 783.99, 0.05),
        ComponentSlot::SectionFail => (320, 392.0, 220.0, 0.20),
        ComponentSlot::MenuClick => (35, 950.0, 600.0, 0.10),
        ComponentSlot::MenuHit => (75, 660.0, 880.0, 0.10),
        ComponentSlot::MenuBack => (90, 540.0, 360.0, 0.08),
        ComponentSlot::Applause => (450, 587.33, 880.0, 0.12),
        ComponentSlot::FailSound => (400, 300.0, 130.0, 0.25),
        ComponentSlot::PauseLoop => (500, 440.0, 440.0, 0.02),
        _ => (60, 500.0, 250.0, 0.10),
    };

    let num_samples = ((sample_rate * duration_ms) / 1000) as usize;
    let mut pcm = Vec::with_capacity(num_samples * 2);
    let mut phase: f32 = 0.0;
    let mut lfsr: u32 = 0xACE1u32;

    for i in 0..num_samples {
        let t = i as f32 / num_samples.max(1) as f32;
        let envelope = (1.0 - t).powf(1.8);
        let freq = base_freq + (end_freq - base_freq) * t;
        phase += std::f32::consts::TAU * freq / sample_rate as f32;
        let tone = phase.sin();

        // Simple deterministic PRNG for percussion/cymbal texture
        lfsr ^= lfsr << 13;
        lfsr ^= lfsr >> 17;
        lfsr ^= lfsr << 5;
        let noise = ((lfsr & 0xFFFF) as f32 / 32768.0) - 1.0;

        let sample_f = (tone * (1.0 - noise_mix) + noise * noise_mix) * envelope * 0.45;
        let sample_i16 = (sample_f.clamp(-1.0, 1.0) * 32767.0) as i16;
        pcm.extend_from_slice(&sample_i16.to_le_bytes());
    }

    let data_len = pcm.len() as u32;
    let riff_len = 36 + data_len;
    let byte_rate = sample_rate * 2;
    let block_align: u16 = 2;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // PCM chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // AudioFormat = 1 (PCM)
    wav.extend_from_slice(&1u16.to_le_bytes()); // NumChannels = 1
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample = 16
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(&pcm);
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use skinforge_core::categorize::ComponentSlot;

    #[test]
    fn synthesized_default_wav_is_valid_decodable_audio_for_all_audio_slots() {
        for &slot in ComponentSlot::all() {
            if slot.is_audio() {
                let bytes = synthesize_default_wav(slot);
                assert!(bytes.len() > 44);
                assert_eq!(&bytes[0..4], b"RIFF");
                assert_eq!(&bytes[8..12], b"WAVE");
                let decoder = Decoder::new(Cursor::new(bytes));
                assert!(
                    decoder.is_ok(),
                    "Failed to decode synthesized WAV for {slot:?}"
                );
            }
        }
    }

    #[test]
    fn volume_clamps_between_zero_and_one() {
        let player = AudioPlayer {
            _stream: None,
            stream_handle: None,
            active_sink: Mutex::new(None),
            volume: Mutex::new(0.7),
        };
        assert!((player.volume() - 0.7).abs() < f32::EPSILON);
        player.set_volume(1.5);
        assert!((player.volume() - 1.0).abs() < f32::EPSILON);
        player.set_volume(-0.5);
        assert!((player.volume() - 0.0).abs() < f32::EPSILON);
    }
}
