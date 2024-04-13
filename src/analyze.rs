use colored::Colorize;
use crate::Color;

pub(crate) fn analyze_frame(frame_data: Vec<u8>, prev_state: Vec<u8>, tolerance: &u8, width: usize, height: usize, x_mid: &usize, y_mid: &usize, offset: &usize, active: &mut bool, smoke_color: &Color) -> bool {
    let mut index = (y_mid * width + x_mid) * 4;

    // FLASH DETECTION
    if frame_data[4 * (10 * width + 10)] > 180 && frame_data[4 * (10 * width + 10) + 1] > 180 && frame_data[4 * (10 * width + 10) + 2] > 180 && frame_data[4 * (10 * width + width - 10)] > 180 && frame_data[4 * (10 * width + width - 10) + 1] > 180 && frame_data[4 * (10 * width + width - 10) + 2] > 180 && frame_data[index] > 180  && frame_data[index + 1] > 180  && frame_data[index + 2] > 180
    {
        return true;
    }

    index = index + offset;

    // SMOKE DETECTION
    if close_colors(&Color{ r: frame_data[index - 198], g: frame_data[index - 199], b: frame_data[index - 200] }, smoke_color, &35_u8) && close_colors(&Color{ r: frame_data[index + 2], g: frame_data[index + 1], b: frame_data[index] }, smoke_color, &35_u8) && close_colors(&Color{ r: frame_data[index + 202], g: frame_data[index + 201], b: frame_data[index + 200] }, smoke_color, &35_u8)
    {
        println!("{} smoke probably detected", "[*]".yellow());

        let off = (offset / (height / 2)) * 20;
        index = index + off - width*20;
        let diagonal_color = Color { r: frame_data[index - 78], g: frame_data[index - 79], b: frame_data[index - 80] };
        if compare_rgb(Color{ r: prev_state[index - 78], g: prev_state[index - 79], b: prev_state[index - 80] }, &diagonal_color, &20_u8) && close_colors(&diagonal_color, smoke_color, &45_u8)
        {
            println!("{} trigger {} automatically {}", "[*]".yellow(), "OFF".red(), "[SMOKE]".yellow());
            *active = false;
            return true;
        }
    }

    // TRIGGER DETECTION
    let mut same = true;
    let mut off_mod = -1_i32;
    let mut offset = *offset;
    for y in y_mid-1.. y_mid+2 {
        for x in x_mid-1..x_mid+2 {
            let index = (y * width + x) * 4;  // Calculate the index of the pixel in the byte slice
            if offset as i32 > 0_i32 { offset = (offset as i32 + off_mod) as usize; }

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

fn close_colors(rgb_color1: &Color, rgb_color2: &Color, difference: &u8) -> bool {
    if u8::abs_diff(rgb_color1.r, rgb_color2.r) < *difference && u8::abs_diff(rgb_color1.g, rgb_color2.g) < *difference && u8::abs_diff(rgb_color1.b, rgb_color2.b) < *difference {
        return true;
    }
    false
}