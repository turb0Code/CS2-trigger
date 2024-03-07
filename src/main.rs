extern crate scrap;
use device_query::{DeviceQuery, DeviceState, Keycode};
use scrap::{Capturer, Display};
use enigo::*;
use std::io::ErrorKind::WouldBlock;
use std::time::{Duration, Instant};
use std::clone::Clone;
use std::thread;

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

fn main() {

    println!("\n[*] CS2 TRIGGER");
    println!("[*] OFF BY DEFAULT");

    let display = Display::primary().expect("Couldn't find main display.");
    let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
    let width = capturer.width();

    let frame_data = capture_frame(&mut capturer);
    let mut state = frame_data.clone();

    let device_state = DeviceState::new();
    let mut last_keys = device_state.get_keys();

    let mut enigo = Enigo::new();

    let mut active = false;
    let mut same = false;
    let mut frame_data: Vec<u8>;

    println!("[+] set up all components \n");

    loop {
        let start = Instant::now();  // STARTED MEASURING

        frame_data = capture_frame(&mut capturer);
        same = analyze_frame(frame_data.clone(), width, state.clone());
        state = frame_data;

        let duration = start.elapsed();  // MEASURE DONE

        let keys = device_state.get_keys();
        if keys != last_keys {
            for key in &keys {
                match key {
                    Keycode::L => {
                        active = true;
                        println!("\n[*] trigger ON");
                    }
                    Keycode::K => {
                        active = false;
                        println!("[*] trigger OFF");
                    }
                    Keycode::A => {
                        if active {
                            active = false;
                            println!("[*] trigger OFF automatically [A]");
                        }
                    }
                    Keycode::D => {
                        if active {
                            active = false;
                            println!("[*] trigger OFF automatically [D]");
                        }
                    }
                    Keycode::Space => {
                        if active {
                            active = false;
                            println!("[*] trigger OFF automatically [Space]");
                        }
                    }
                    _ => {}
                }
            }
            last_keys = keys;
        }

        if !same && active {
            enigo.mouse_click(MouseButton::Left);
            println!("\n[+] CLICKED");
            println!("[*] elapsed time: {:?}", duration);  // SHOWED ELAPSED TIME
            println!("[*] trigger OFF");
            active = false;
        }
    }

}

fn analyze_frame(frame_data: Vec<u8>, width: usize, prev_state: Vec<u8>) -> bool {
    let mut same = true;
    for y in 538.. 542 {
        for x in 959..961 {
            let index = (y * width + x) * 4;  // Calculate the index of the pixel in the byte slice

            let prev_color = Color{ r: prev_state[index+2], g: prev_state[index+1], b: prev_state[index] };
            let cur_color = Color{ r: frame_data[index + 2], g: frame_data[index + 1], b: frame_data[index] };

            same = compare_rgb(prev_color, cur_color, 30);
        }
    }
    same
}

fn compare_rgb(rgb_old: Color, rgb_new: Color, tolerance: u8) -> bool {
    if u8::abs_diff(rgb_old.r, rgb_new.r) > tolerance && u8::abs_diff(rgb_old.g, rgb_new.g) > tolerance && u8::abs_diff(rgb_old.b, rgb_new.b) > tolerance {
        return false;
    }
    true
}

fn capture_frame(capturer: &mut Capturer) -> Vec<u8> {
    loop {
        match capturer.frame() {
            Ok(frame) => {
                return (&*frame).to_vec();
            },
            Err(ref e) if e.kind() == WouldBlock => {
                thread::sleep(Duration::from_millis(1));
                continue;
            },
            Err(e) => panic!("Error: {}", e),
        }
    }
}
