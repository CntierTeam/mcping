//! Render a Java favicon PNG as colored half-block ASCII art.

use base64::Engine;
use png::{ColorType, Decoder};
use std::io::Cursor;

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

fn render(png_bytes: &[u8], size: u32) -> Result<String, ()> {
    let (w0, h0, rgba) = decode_rgba(png_bytes)?;
    let rgba = resize_nearest(&rgba, w0, h0, size, size);
    // Per-cell SGR + reset is ~30–40 bytes; size for worst case.
    let mut out = String::with_capacity((size as usize) * (size as usize) * 40);
    let h = size;
    let w = size;
    let mut y = 0;
    while y < h {
        for x in 0..w {
            let top = pixel(&rgba, w, x, y);
            let bottom = if y + 1 < h {
                pixel(&rgba, w, x, y + 1)
            } else {
                [0, 0, 0, 0]
            };
            out.push_str(&half_block(top, bottom));
        }
        // End-of-row reset so a following cell/line never inherits fg/bg.
        out.push_str("\x1b[0m\n");
        y += 2;
    }
    // Whole-icon reset before MOTD/reply/normal output.
    out.push_str("\x1b[0m");
    Ok(out)
}

fn decode_rgba(png_bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), ()> {
    let mut decoder = Decoder::new(Cursor::new(png_bytes));
    // Favicons are tiny; expand palette / gray and strip 16-bit so we only handle 8-bit.
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|_| ())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or(())?];
    let info = reader.next_frame(&mut buf).map_err(|_| ())?;
    let w = info.width;
    let h = info.height;
    let rgba = match info.color_type {
        ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        ColorType::Rgb => {
            let rgb = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity((rgb.len() / 3) * 4);
            for chunk in rgb.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        ColorType::Grayscale => {
            let g = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity(g.len() * 4);
            for &v in g {
                out.extend_from_slice(&[v, v, v, 255]);
            }
            out
        }
        ColorType::GrayscaleAlpha => {
            let ga = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity((ga.len() / 2) * 4);
            for chunk in ga.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        ColorType::Indexed => return Err(()),
    };
    Ok((w, h, rgba))
}

fn resize_nearest(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            let si = ((sy * sw + sx) * 4) as usize;
            let di = ((y * dw + x) * 4) as usize;
            out[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    out
}

fn pixel(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn half_block(top: [u8; 4], bottom: [u8; 4]) -> String {
    const RESET: &str = "\x1b[0m";
    let ta = top[3];
    let ba = bottom[3];
    // Fully transparent pair → space (no open SGR to bleed into neighbors)
    if ta < 16 && ba < 16 {
        return " ".to_string();
    }
    // Use ▀ with FG=top, BG=bottom. Always reset after the glyph so the next
    // cell cannot inherit a dangling 24-bit fg/bg (especially bg after a ▀ that
    // only sets fg for a transparent neighbor).
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
        return format!("\x1b[38;2;{br};{bg};{bb}m▄{RESET}");
    }
    if ba < 16 {
        return format!("\x1b[38;2;{tr};{tg};{tb}m▀{RESET}");
    }
    format!("\x1b[38;2;{tr};{tg};{tb}m\x1b[48;2;{br};{bg};{bb}m▀{RESET}")
}
