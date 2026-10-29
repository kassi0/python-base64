use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base64Mode {
    Standard,
    UrlSafe,
}

/// Encode bytes to Base64 string
pub fn encode(data: &[u8], urlsafe: bool) -> String {
    if urlsafe {
        URL_SAFE.encode(data)
    } else {
        STANDARD.encode(data)
    }
}

/// Helper to fix missing padding in base64 strings
pub fn fix_padding(input: &str) -> String {
    let mut s = input.trim().to_string();
    let pad_len = (4 - (s.len() % 4)) % 4;
    for _ in 0..pad_len {
        s.push('=');
    }
    s
}

/// Clean input string (remove whitespace/newlines) and fix padding
pub fn sanitize_b64(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| *c != '\r' && *c != '\n' && *c != '\t')
        .collect();
    fix_padding(cleaned.trim())
}

/// Decode a Base64 string.
/// Attempts standard decode, urlsafe decode, or auto-detect.
pub fn decode(input: &str, urlsafe_hint: bool) -> Result<Vec<u8>, String> {
    let sanitized = sanitize_b64(input);

    if sanitized.is_empty() {
        return Ok(Vec::new());
    }

    if urlsafe_hint {
        if let Ok(bytes) = URL_SAFE.decode(&sanitized) {
            return Ok(bytes);
        }
        if let Ok(bytes) = STANDARD.decode(&sanitized) {
            return Ok(bytes);
        }
    } else {
        if let Ok(bytes) = STANDARD.decode(&sanitized) {
            return Ok(bytes);
        }
        // If string contains spaces (common in url-decoded form submissions where '+' becomes ' '), try with '+'
        if sanitized.contains(' ') {
            let with_plus = sanitized.replace(' ', "+");
            if let Ok(bytes) = STANDARD.decode(&with_plus) {
                return Ok(bytes);
            }
        }
        // Also try stripping remaining spaces
        let stripped: String = sanitized.chars().filter(|c| !c.is_whitespace()).collect();
        if let Ok(bytes) = STANDARD.decode(&fix_padding(&stripped)) {
            return Ok(bytes);
        }
        if let Ok(bytes) = URL_SAFE.decode(&sanitized) {
            return Ok(bytes);
        }
    }

    Err("Base64 inválido ou mal formatado.".to_string())
}

/// Decode raw bytes directly
#[allow(dead_code)]
pub fn decode_bytes(bytes: &[u8], urlsafe_hint: bool) -> Result<Vec<u8>, String> {
    let s = String::from_utf8_lossy(bytes);
    decode(&s, urlsafe_hint)
}

/// Detect and strip data URL prefix (e.g. data:image/png;base64,iVBORw0KGgo...)
pub fn strip_data_url_prefix(input: &str) -> (&str, Option<String>) {
    let trimmed = input.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("data:") {
        if let Some(pos) = lower.find(";base64,") {
            let mime_part = trimmed[5..pos].trim();
            let data_part = &trimmed[pos + 8..];
            let mime = if mime_part.is_empty() {
                None
            } else {
                Some(mime_part.to_string())
            };
            return (data_part, mime);
        }
    }
    (trimmed, None)
}

/// Guess appropriate download filename based on original filename and action
pub fn guess_download_name(original: Option<&str>, action: &str, detected_ext: Option<&str>) -> String {
    let original = original.unwrap_or("resultado");
    let base = original.rsplit('/').next().unwrap_or(original);
    let base = base.rsplit('\\').next().unwrap_or(base);

    if action == "encode" {
        format!("{base}.b64.txt")
    } else {
        if base.contains(".b64.") {
            return base.replace(".b64.", ".dec.");
        }
        if base.ends_with(".b64") || base.ends_with(".txt") {
            if let Some(stripped) = base.strip_suffix(".b64") {
                return stripped.to_string();
            }
            if let Some(stripped) = base.strip_suffix(".txt") {
                return stripped.to_string();
            }
        }
        if let Some(ext) = detected_ext {
            if !ext.is_empty() {
                return format!("{base}{ext}");
            }
        }
        format!("{base}.decoded")
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileTypeInfo {
    pub extension: &'static str,
    pub mime: &'static str,
    pub label: &'static str,
    pub is_image: bool,
    pub is_audio: bool,
}

/// Detect file format by magic bytes signature
pub fn detect_file_type(bytes: &[u8]) -> FileTypeInfo {
    if bytes.starts_with(b"%PDF-") {
        return FileTypeInfo {
            extension: ".pdf",
            mime: "application/pdf",
            label: "PDF Document",
            is_image: false,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return FileTypeInfo {
            extension: ".png",
            mime: "image/png",
            label: "PNG Image",
            is_image: true,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return FileTypeInfo {
            extension: ".jpg",
            mime: "image/jpeg",
            label: "JPEG Image",
            is_image: true,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return FileTypeInfo {
            extension: ".gif",
            mime: "image/gif",
            label: "GIF Image",
            is_image: true,
            is_audio: false,
        };
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return FileTypeInfo {
            extension: ".webp",
            mime: "image/webp",
            label: "WebP Image",
            is_image: true,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return FileTypeInfo {
            extension: ".zip",
            mime: "application/zip",
            label: "ZIP Archive",
            is_image: false,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"BM") {
        return FileTypeInfo {
            extension: ".bmp",
            mime: "image/bmp",
            label: "BMP Image",
            is_image: true,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"MZ") {
        return FileTypeInfo {
            extension: ".exe",
            mime: "application/octet-stream",
            label: "Windows Executable (PE)",
            is_image: false,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"\x7fELF") {
        return FileTypeInfo {
            extension: ".elf",
            mime: "application/octet-stream",
            label: "Linux Executable (ELF)",
            is_image: false,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"ID3") || bytes.starts_with(b"\xff\xfb") || bytes.starts_with(b"\xff\xfa") {
        return FileTypeInfo {
            extension: ".mp3",
            mime: "audio/mpeg",
            label: "MP3 Audio",
            is_image: false,
            is_audio: true,
        };
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return FileTypeInfo {
            extension: ".wav",
            mime: "audio/wav",
            label: "WAV Audio",
            is_image: false,
            is_audio: true,
        };
    }
    if bytes.starts_with(b"OggS") {
        return FileTypeInfo {
            extension: ".ogg",
            mime: "audio/ogg",
            label: "OGG Audio",
            is_image: false,
            is_audio: true,
        };
    }
    if bytes.starts_with(b"\x1f\x8b") {
        return FileTypeInfo {
            extension: ".gz",
            mime: "application/gzip",
            label: "GZIP Archive",
            is_image: false,
            is_audio: false,
        };
    }
    if bytes.starts_with(b"7z\xbc\xaf\x27\x1c") {
        return FileTypeInfo {
            extension: ".7z",
            mime: "application/x-7z-compressed",
            label: "7-Zip Archive",
            is_image: false,
            is_audio: false,
        };
    }

    FileTypeInfo {
        extension: ".bin",
        mime: "application/octet-stream",
        label: "Binário Genérico",
        is_image: false,
        is_audio: false,
    }
}

/// Format bytes as structured hexdump lines with offset, hex bytes and ASCII
#[allow(dead_code)]
pub fn generate_hexdump(data: &[u8], max_bytes: usize) -> String {
    let slice = if data.len() > max_bytes {
        &data[..max_bytes]
    } else {
        data
    };

    let mut out = String::new();
    for (i, chunk) in slice.chunks(16).enumerate() {
        let offset = i * 16;
        let mut hex_part = String::new();
        let mut ascii_part = String::new();

        for j in 0..16 {
            if j < chunk.len() {
                let b = chunk[j];
                hex_part.push_str(&format!("{b:02x} "));
                if j == 7 {
                    hex_part.push(' ');
                }
                if b.is_ascii_graphic() || b == b' ' {
                    ascii_part.push(b as char);
                } else {
                    ascii_part.push('.');
                }
            } else {
                hex_part.push_str("   ");
                if j == 7 {
                    hex_part.push(' ');
                }
            }
        }

        out.push_str(&format!("{offset:08x}  {hex_part} |{ascii_part}|\n"));
    }

    if data.len() > max_bytes {
        out.push_str(&format!("... [truncado, total de {} bytes]\n", data.len()));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_and_decode_roundtrip() {
        let original = "Olá, Base64 em Rust! 🦀";
        let encoded = encode(original.as_bytes(), false);
        let decoded = decode(&encoded, false).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), original);
    }

    #[test]
    fn test_urlsafe_encode_decode() {
        let data = vec![0xfb, 0xff, 0xfe]; // contains bytes that produce + and / in standard
        let enc_std = encode(&data, false);
        let enc_url = encode(&data, true);
        assert!(enc_std.contains('+') || enc_std.contains('/'));
        assert!(!enc_url.contains('+') && !enc_url.contains('/'));

        let dec = decode(&enc_url, true).unwrap();
        assert_eq!(dec, data);
    }

    #[test]
    fn test_auto_padding() {
        // "YW55IGNhcm5hbCBwbGVhc3VyZS4" is "any carnal pleasure." without trailing '='
        let missing_pad = "YW55IGNhcm5hbCBwbGVhc3VyZS4";
        let dec = decode(missing_pad, false).unwrap();
        assert_eq!(String::from_utf8(dec).unwrap(), "any carnal pleasure.");
    }

    #[test]
    fn test_strip_data_url() {
        let data_url = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB";
        let (data, mime) = strip_data_url_prefix(data_url);
        assert_eq!(mime, Some("image/png".to_string()));
        assert_eq!(data, "iVBORw0KGgoAAAANSUhEUgAAAAEAAAAB");

        let plain = "SGVsbG8=";
        let (data2, mime2) = strip_data_url_prefix(plain);
        assert_eq!(mime2, None);
        assert_eq!(data2, "SGVsbG8=");
    }

    #[test]
    fn test_magic_detection() {
        let png_bytes = b"\x89PNG\r\n\x1a\n\x00\x00";
        let info = detect_file_type(png_bytes);
        assert_eq!(info.extension, ".png");
        assert_eq!(info.mime, "image/png");
        assert!(info.is_image);

        let pdf_bytes = b"%PDF-1.4\n...";
        let info_pdf = detect_file_type(pdf_bytes);
        assert_eq!(info_pdf.extension, ".pdf");
        assert_eq!(info_pdf.mime, "application/pdf");
    }

    #[test]
    fn test_hexdump() {
        let sample = b"Hello World 1234";
        let dump = generate_hexdump(sample, 64);
        assert!(dump.contains("Hello World 1234"));
    }

    #[test]
    fn test_decode_bytes() {
        let b = b"SGVsbG8=";
        let dec = decode_bytes(b, false).unwrap();
        assert_eq!(String::from_utf8(dec).unwrap(), "Hello");
    }
}
