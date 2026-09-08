//! Minecraft `§` / legacy color codes → ANSI escape sequences.

const RESET: &str = "\x1b[0m";

fn color_fg(code: char) -> Option<&'static str> {
    Some(match code {
        '0' => "\x1b[30m",           // black
        '1' => "\x1b[34m",           // dark blue
        '2' => "\x1b[32m",           // dark green
        '3' => "\x1b[36m",           // dark aqua
        '4' => "\x1b[31m",           // dark red
        '5' => "\x1b[35m",           // dark purple
        '6' => "\x1b[33m",           // gold
        '7' => "\x1b[37m",           // gray
        '8' => "\x1b[90m",           // dark gray
        '9' => "\x1b[94m",           // blue
        'a' | 'A' => "\x1b[92m",     // green
        'b' | 'B' => "\x1b[96m",     // aqua
        'c' | 'C' => "\x1b[91m",     // red
        'd' | 'D' => "\x1b[95m",     // light purple
        'e' | 'E' => "\x1b[93m",     // yellow
        'f' | 'F' => "\x1b[97m",     // white
        _ => return None,
    })
}

fn format_code(code: char) -> Option<&'static str> {
    Some(match code {
        'l' | 'L' => "\x1b[1m",  // bold
        'n' | 'N' => "\x1b[4m",  // underline
        'o' | 'O' => "\x1b[3m",  // italic
        'm' | 'M' => "\x1b[9m",  // strikethrough
        'k' | 'K' => "",         // obfuscated — ignore in terminal
        'r' | 'R' => RESET,
        _ => return None,
    })
}

/// Convert a legacy-coded MOTD string to ANSI-colored text.
pub fn to_ansi(motd: &str) -> String {
    let chars: Vec<char> = motd.chars().collect();
    let mut out = String::with_capacity(motd.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == '§' || chars[i] == '&') && i + 1 < chars.len() {
            let code = chars[i + 1];
            if let Some(ansi) = color_fg(code).or_else(|| format_code(code)) {
                out.push_str(ansi);
                i += 2;
                continue;
            }
        }
        // Normalize newlines often present in MOTDs
        if chars[i] == '\n' {
            out.push('\n');
        } else {
            out.push(chars[i]);
        }
        i += 1;
    }
    out.push_str(RESET);
    out
}

/// Strip formatting codes for plain-text contexts.
#[allow(dead_code)]
pub fn strip(motd: &str) -> String {
    let chars: Vec<char> = motd.chars().collect();
    let mut out = String::with_capacity(motd.len());
    let mut i = 0;
    while i < chars.len() {
        if (chars[i] == '§' || chars[i] == '&') && i + 1 < chars.len() {
            let c = chars[i + 1];
            if color_fg(c).is_some() || format_code(c).is_some() {
                i += 2;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
