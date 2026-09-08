//! Render a Java favicon PNG as colored half-block ASCII art.

use base64::Engine;
use image::imageops::FilterType;
use image::RgbaImage;

const DEFAULT_SIZE: u32 = 32;
const DATA_URL_PREFIX: &str = "data:image/png;base64,";

/// Print favicon data-URL to stdout as ANSI half-block art (~32×32).
pub fn print_ascii(data_url: &str) {
    match decode_png(data_url).and_then(|png| render(&png, DEFAULT_SIZE)) {
        Ok(s) => print!("{s}"),
        Err(_) => {
            // Silent skip — favicon is optional eye candy.
        }
    }
}

fn decode_png(data_url: &str) -> Result<Vec<u8>, ()> {
    let b64 = data_url.strip_prefix(DATA_URL_PREFIX).ok_or(())?;
    base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| ())
}

fn render(png: &[u8], size: u32) -> Result<String, ()> {
    let img = image::load_from_memory(png).map_err(|_| ())?;
    // Nearest is plenty for 64→32 favicon downscale and cheaper than Triangle.
    let rgba: RgbaImage = img
        .resize_exact(size, size, FilterType::Nearest)
        .to_rgba8();
    let mut out = String::with_capacity((size as usize) * (size as usize) * 24);
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
