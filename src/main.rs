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

    let display = Display::primary().expect("[-] couldn't find main display");
    let mut capturer = Capturer::new(display).expect("[-] couldn't begin capture");
    let width = capturer.width();
    let height = capturer.height();

    let mut active = false;
    let mut same = false;
    let mut frame_data: Vec<u8>;
    let y_mid = capturer.height()/2;
    let x_mid = &width/2;
    let tolerance = 30_u8;
    let mut offset = 0_usize;

    frame_data = capture_frame(&mut capturer);
    let mut state = frame_data.clone();

    let device_state = DeviceState::new();
    let mut last_keys = device_state.get_keys();
    let mut enigo = Enigo::new();

    if frame_data.len() > (width * height * 4) {
        let diff = frame_data.len() - (width * height * 4);
        let cols = diff/height;
        offset = cols * height/2;
        println!("[*] data len: {} | size: {}", frame_data.len(), width * height * 4);
        println!("[*] diff: {} | cols: {}", diff, cols);
        println!("[+] calculated offset: {}", offset);
    }

    println!("[*] tolerance set to {}", tolerance);
    println!("[+] set up all components \n");

    loop {
        let start = Instant::now();  // STARTED MEASURING

        frame_data = capture_frame(&mut capturer);
        same = analyze_frame(frame_data.clone(), state.clone(), &tolerance, width, height, &x_mid, &y_mid, &offset, &mut active);
        state = frame_data;

        let duration = start.elapsed();  // MEASURE DONE

        let keys = device_state.get_keys();
        if keys != last_keys {
            for key in &keys {
                if active {
                    match key {
                        Keycode::K => {
                            active = false;
                            println!("[*] trigger OFF");
                        }
                        Keycode::A => {
                            active = false;
                            println!("[*] trigger OFF automatically [A]");
                        }
                        Keycode::D => {
                            active = false;
                            println!("[*] trigger OFF automatically [D]");
                        }
                        Keycode::Space => {
                            active = false;
                            println!("[*] trigger OFF automatically [Space]");
                        }
                        _ => {}
                    }
                }
                else {
                    match key {
                        Keycode::L => {
                            active = true;
                            println!("\n[*] trigger ON");
                        }
                        _ => {}
                    }
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

fn analyze_frame(frame_data: Vec<u8>, prev_state: Vec<u8>, tolerance: &u8, width: usize, height: usize, x_mid: &usize, y_mid: &usize, offset: &usize, active: &mut bool) -> bool {
    let index = (y_mid * width + x_mid) * 4;
    // let smoke_color = Color{ r: 110,  g: 106, b: 98 };  // CT COLOR
    // let smoke_color = Color{ r: 104, g: 94, b: 71 };  // TT COLOR
    let smoke_color = Color{ r: 107, g: 100, b: 85 };  // MAIN COLOR
    if frame_data[4 * (10 * width + 10)] > 180 && frame_data[4 * (10 * width + 10) + 1] > 180 && frame_data[4 * (10 * width + 10) + 2] > 180 && frame_data[4 * (10 * width + width - 10)] > 180 && frame_data[4 * (10 * width + width - 10) + 1] > 180 && frame_data[4 * (10 * width + width - 10) + 2] > 180 && frame_data[index] > 180  && frame_data[index + 1] > 180  && frame_data[index + 2] > 180
    {
        return true;
    }
    if close_colors(&Color{ r: frame_data[index + offset + 2 - 200], g: frame_data[index + offset + 1 - 200], b: frame_data[index + offset - 200] }, &smoke_color, &35_u8) && close_colors(&Color{ r: frame_data[index + offset + 2], g: frame_data[index + offset + 1], b: frame_data[index + offset] }, &smoke_color, &35_u8) && close_colors(&Color{ r: frame_data[index + offset + 2 + 200], g: frame_data[index + offset + 1 + 200], b: frame_data[index + offset + 200] }, &smoke_color, &35_u8)
    {
        if *active
        {
            println!("[+] smoke probably detected");
        }
        let off = (offset / (height / 2)) * 20;
        if compare_rgb(Color{ r: prev_state[index + off + 2 - width*20 - 80], g: prev_state[index + off + 1 - width*20 - 80], b: prev_state[index + off - width*20 - 80] }, &Color { r: frame_data[index + off + 2 - width*20 - 80], g: frame_data[index + off +  1 - width*20 - 80], b: frame_data[index + off - width*20 - 80] }, &20_u8) && close_colors(&Color{ r: frame_data[index + off + 2 - width*20 - 80], g: frame_data[index + off + 1 - width*20 - 80], b: frame_data[index + off - width*20 - 80] }, &smoke_color, &45_u8)
        {
            if *active
            {
                *active = false;
                println!("[*] trigger OFF automatically [SMOKE]");
            }
            return true;
        }
    }
    let mut same = true;
    let mut off_mod = -1_i32;
    for y in y_mid-1.. y_mid+2 {
        for x in x_mid-1..x_mid+2 {
            let index = (y * width + x) * 4;  // Calculate the index of the pixel in the byte slice
            if *offset as i32 > 0_i32 { let offset = *offset as i32 + off_mod; }

            let prev_color = Color{ r: prev_state[index + offset + 2], g: prev_state[index + offset+ 1], b: prev_state[index + offset] };
            let cur_color = Color{ r: frame_data[index + offset + 2], g: frame_data[index + offset + 1], b: frame_data[index + offset] };

            same = compare_rgb(prev_color, &cur_color, tolerance);
        }
        off_mod += 1;
    }
    same
}

fn compare_rgb(rgb_old: Color, rgb_new: &Color, tolerance: &u8) -> bool {
    if u8::abs_diff(rgb_old.r, rgb_new.r) > *tolerance && u8::abs_diff(rgb_old.g, rgb_new.g) > *tolerance && u8::abs_diff(rgb_old.b, rgb_new.b) > *tolerance {
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
            Err(e) => panic!("[-] error: {}", e),
        }
    }
}

fn close_colors(rgb_color1: &Color, rgb_color2: &Color, difference: &u8) -> bool {
    if u8::abs_diff(rgb_color1.r, rgb_color2.r) < *difference && u8::abs_diff(rgb_color1.g, rgb_color2.g) < *difference && u8::abs_diff(rgb_color1.b, rgb_color2.b) < *difference {
        return true;
    }
    false
}