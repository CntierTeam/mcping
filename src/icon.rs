//! Render a Java favicon PNG as colored half-block ASCII art.

use image::imageops::FilterType;
use image::RgbaImage;

const DEFAULT_SIZE: u32 = 32;

/// Print favicon to stdout as ANSI half-block art (~32×32).
pub fn print_ascii(png: &[u8]) {
    match render(png, DEFAULT_SIZE) {
        Ok(s) => print!("{s}"),
        Err(_) => {
            // Silent skip — favicon is optional eye candy.
        }
    }
}

fn render(png: &[u8], size: u32) -> Result<String, image::ImageError> {
    let img = image::load_from_memory(png)?;
    let rgba: RgbaImage = img
        .resize_exact(size, size, FilterType::Triangle)
        .to_rgba8();
    let mut out = String::new();
    let h = rgba.height();
    let w = rgba.width();
    let mut y = 0;
    while y < h {
        for x in 0..w {
            let top = rgba.get_pixel(x, y).0;
            let bottom = if y + 1 < h {
                rgba.get_pixel(x, y + 1).0
            } else {
                [0, 0, 0, 0]
            };
            out.push_str(&half_block(top, bottom));
        }
        out.push_str("\x1b[0m\n");
        y += 2;
    }
    Ok(out)
}

fn half_block(top: [u8; 4], bottom: [u8; 4]) -> String {
    let ta = top[3];
    let ba = bottom[3];
    // Fully transparent pair → space
    if ta < 16 && ba < 16 {
        return " ".to_string();
    }
    // Use ▀ with FG=top, BG=bottom
    let (tr, tg, tb) = if ta < 16 {
        (0u8, 0u8, 0u8)
    } else {
        (top[0], top[1], top[2])
    };
    let (br, bg, bb) = if ba < 16 {
        (0u8, 0u8, 0u8)
    } else {
        (bottom[0], bottom[1], bottom[2])
    };
    if ta < 16 {
        // only bottom visible: use ▄ with FG=bottom
        return format!("\x1b[38;2;{br};{bg};{bb}m▄");
    }
    if ba < 16 {
        return format!("\x1b[38;2;{tr};{tg};{tb}m▀");
    }
    format!("\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀")
}
