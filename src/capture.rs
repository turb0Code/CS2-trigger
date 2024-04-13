use crate::DirectXCapture::OutCapturer;
use std::io::ErrorKind::WouldBlock;
use std::time::{Duration};
use std::{io, thread};

pub(crate) fn capture_frame(capturer: &mut OutCapturer) -> Vec<u8> {
    loop {
        match capturer.frame() {
            Ok(frame) => {
                return frame.to_vec();
            },
            Err(ref e) if e.kind() == WouldBlock => {
                thread::sleep(Duration::from_micros(500));
                continue;
            },
            Err(e) => {}
        }
    }
}

