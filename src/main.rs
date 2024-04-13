mod analyze;
mod capture;
mod DirectXCapture;

use DirectXCapture::{OutCapturer, OutDisplay};
use capture::capture_frame;
use analyze::analyze_frame;

use rodio::{source::SineWave, OutputStream, Sink, Source};
use device_query::{DeviceQuery, DeviceState, Keycode};
use scrap::{Capturer, Display};
use colored::Colorize;
use enigo::*;

use std::io::ErrorKind::WouldBlock;
use std::time::{Duration, Instant};
use std::process::exit;
use std::clone::Clone;
use std::ops::Deref;
use std::thread;

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

fn main() {
    println!("{} {}", "\n[*]".yellow(), "CS2 TRIGGER".bold().cyan());
    println!("{} {} BY DEFAULT", "[*]".yellow(), "OFF".red());

    // SOUND SYSTEM
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    let sound_source_on = SineWave::new(440.0).take_duration(Duration::from_secs_f32(0.20)).amplify(0.07);
    let sound_source_off = SineWave::new(340.0).take_duration(Duration::from_secs_f32(0.20)).amplify(0.07);

    // CAPTURE SETUP
    let display = unsafe { OutDisplay::primary() }.expect("[-] couldn't find main display");
    let mut capturer = OutCapturer::new(display).expect("[-] couldn't begin capture");
    let width = capturer.width();
    let height = capturer.height();

    // ANALYSIS VARIABLES
    let mut active = false;
    let mut same = true;
    let mut frame_data: Vec<u8> = vec![];
    let y_mid = capturer.height() - 3;
    let x_mid = &width / 2;
    let tolerance = 30_u8;
    let mut offset = 0_usize;
    let smoke_color = &Color { r: 100, g: 93, b: 78 };  // MAIN COLOR
    // let smoke_color = &Color{ r: 110,  g: 106, b: 98 };      // CT COLOR
    // let smoke_color = &Color{ r: 104, g: 94, b: 71 };        // TT COLOR

    // FIRST CAPTURE
    frame_data = capture_frame(&mut capturer);
    let mut state = frame_data.clone();

    // KEY READING SETUP
    let device_state = DeviceState::new();
    let mut last_keys = device_state.get_keys();
    let mut enigo = Enigo::new();

    // CALCULATE OFFSET (if needed)
    if frame_data.len() > (width * height * 4) {
        let diff = frame_data.len() - (width * height * 4);
        let cols = diff / height;
        offset = cols * height;
        println!("{} data len: {} | size: {}", "[*]".yellow(), frame_data.len(), width * height * 4);
        println!("{} diff: {} | cols: {}", "[*]".yellow(), diff, cols);
        println!("{} calculated offset: {}", "[+]".green(), offset.to_string().bold().yellow());
    }

    println!("{} tolerance set to {}", "[*]".yellow(), tolerance.to_string().bold().yellow());
    println!("{} set up all components \n", "[+]".green());
    println!("{} PRESS {} -> trigger ON \n    PRESS {} -> trigger OFF", "[*]".yellow(), "[L]".yellow(), "[K]".yellow());

    // MAIN LOOP
    loop {
        let keys = device_state.get_keys();  // READ PRESSED KEYS
        if keys != last_keys {
            for key in &keys {
                if active {
                    match key {
                        Keycode::K => {
                            sink.append(sound_source_off.clone());
                            active = false;
                            println!("{} trigger {}", "[*]".yellow(), "OFF".red());
                        }
                        Keycode::A => {
                            sink.append(sound_source_off.clone());
                            active = false;
                            println!("{} trigger {} automatically {}", "[*]".yellow(), "OFF".red(), "[A]".yellow());
                        }
                        Keycode::D => {
                            sink.append(sound_source_off.clone());
                            active = false;
                            println!("{} trigger {} automatically {}", "[*]".yellow(), "OFF".red(), "[D]".yellow());
                        }
                        Keycode::Space => {
                            sink.append(sound_source_off.clone());
                            active = false;
                            println!("{} trigger {} automatically {}", "[*]".yellow(), "OFF".red(), "[Space]".yellow());
                        }
                        _ => {}
                    }
                } else {
                    match key {
                        Keycode::L => {
                            sink.append(sound_source_on.clone());
                            frame_data = capture_frame(&mut capturer);
                            state = frame_data.clone();
                            active = true;
                            println!("\n{} trigger {}", "[*]".yellow(), "ON".green());
                        }
                        _ => {}
                    }
                }
            }
            last_keys = keys;
        }

        if active {
            let start = Instant::now();  // STARTED MEASURING

            let s_capture = Instant::now();
            frame_data = capture_frame(&mut capturer);
            let e_capture = s_capture.elapsed();

            let s_analyze = Instant::now();
            same = analyze_frame(frame_data.clone(), state.clone(), &tolerance, width, height, &x_mid, &y_mid, &offset, &mut active, smoke_color);
            let e_analyze = s_analyze.elapsed();

            let s_copy = Instant::now();
            state = frame_data.clone();
            let e_copy = s_copy.elapsed();

            let duration = start.elapsed();  // MEASURE DONE

            if !same {
                enigo.mouse_click(MouseButton::Left);
                println!("\n{} {}", "[+]".green(), "CLICKED".bold());
                println!("{} elapsed time: {:?}", "[*]".yellow(), duration);  // SHOW ELAPSED TIME
                println!("CAPTURE: {:?} | ANALYZE: {:?} | COPY: {:?}", e_capture, e_analyze, e_copy);
                println!("{} trigger {}", "[*]".yellow(), "OFF".red());
                active = false;
                sink.append(sound_source_off.clone());
            }
        } else { thread::sleep(Duration::from_millis(10)); }
    }
}