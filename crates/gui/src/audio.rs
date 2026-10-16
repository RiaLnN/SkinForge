use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::path::Path;

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
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
            },
            Err(err) => {
                eprintln!("Failed to initialize audio output stream: {err}");
                Self {
                    _stream: None,
                    stream_handle: None,
                }
            }
        }
    }

    pub fn play_from_file(&self, path: &Path) {
        if let Ok(bytes) = std::fs::read(path) {
            self.play_bytes(bytes);
        }
    }

    pub fn play_bytes(&self, bytes: Vec<u8>) {
        let Some(handle) = &self.stream_handle else {
            return;
        };

        let cursor = Cursor::new(bytes);
        if let Ok(source) = Decoder::new(cursor) {
            if let Ok(sink) = Sink::try_new(handle) {
                sink.append(source);
                sink.detach();
            }
        }
    }
}
