use crate::b64::{self, detect_file_type};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedSecretItem {
    pub key: Option<String>,
    pub original: String,
    pub decoded: String,
    pub is_binary: bool,
    pub detected_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDecodeResult {
    pub ok: bool,
    pub decoded_text: String,
    pub count: usize,
    pub is_k8s_secret: bool,
    pub items: Vec<DecodedSecretItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoEncodeResult {
    pub ok: bool,
    pub encoded_text: String,
    pub count: usize,
    pub is_k8s_secret: bool,
}

/// Check if decoded bytes represent plausible text (mostly printable UTF-8)
fn is_plausible_text(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        let total = text.chars().count();
        if total == 0 {
            return None;
        }
        let printable = text
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .count();
        
        let ratio = printable as f64 / total as f64;
        if ratio >= 0.85 {
            return Some(text);
        }
    }
    None
}

/// Try decoding a candidate Base64 string.
/// Returns (decoded_representation, is_binary, detected_type)
fn try_decode_candidate(candidate: &str) -> Option<(String, bool, String)> {
    let trimmed = candidate.trim().trim_matches(|c| c == '"' || c == '\'');
    if trimmed.len() < 4 {
        return None;
    }

    // Must consist only of base64 chars
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '-' || c == '_' || c == '=') {
        return None;
    }

    if let Ok(bytes) = b64::decode(trimmed, false) {
        if bytes.is_empty() {
            return None;
        }

        // Check if printable text
        if let Some(text) = is_plausible_text(&bytes) {
            return Some((text, false, "Texto UTF-8".to_string()));
        }

        // Check if recognized binary
        let info = detect_file_type(&bytes);
        if info.extension != ".bin" {
            let desc = format!("[Binário: {} ({} bytes)]", info.label, bytes.len());
            return Some((desc, true, info.label.to_string()));
        }
    }

    None
}

/// Quote text safely for YAML if needed
fn format_yaml_value(val: &str) -> String {
    if val.contains('\n') {
        // Multi-line block scalar
        let mut out = "|\n".to_string();
        for line in val.lines() {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
        out.trim_end().to_string()
    } else if val.contains(':') || val.contains(';') || val.contains('#') || val.contains('{') || val.contains('[') || val.contains('"') || val.contains('\'') {
        format!("\"{}\"", val.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        val.to_string()
    }
}

/// Unquote a YAML scalar value
fn unquote_yaml_value(val: &str) -> String {
    let t = val.trim();
    if (t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')) {
        if t.len() >= 2 {
            let inner = &t[1..t.len() - 1];
            return inner.replace("\\\"", "\"").replace("\\\\", "\\");
        }
    }
    t.to_string()
}

/// Process a full text (e.g. Kubernetes Secret YAML, JSON, config, or mixed text),
/// automatically discovering and decoding Base64 tokens.
pub fn auto_decode_text(input: &str, convert_k8s_data_to_string_data: bool) -> AutoDecodeResult {
    let mut items = Vec::new();
    let mut output_lines = Vec::new();
    let mut in_data_section = false;
    let mut data_section_indent = 0;
    let is_k8s_secret = input.contains("kind: Secret") || (input.contains("type: Opaque") && input.contains("data:"));

    for line in input.lines() {
        let trimmed_line = line.trim_start();
        let indent = line.len() - trimmed_line.len();

        // Detect Kubernetes Secret `data:` section
        if trimmed_line.starts_with("data:") {
            in_data_section = true;
            data_section_indent = indent;

            if convert_k8s_data_to_string_data && is_k8s_secret {
                let prefix = &line[..indent];
                output_lines.push(format!("{prefix}stringData:"));
                continue;
            } else {
                output_lines.push(line.to_string());
                continue;
            }
        }

        // Check if we exited `data:` section (another key at same or lesser indent)
        if in_data_section && indent <= data_section_indent && !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
            in_data_section = false;
        }

        // 1. Check for YAML/JSON key-value pattern: `key: value`
        if let Some(colon_pos) = trimmed_line.find(':') {
            let key_part = trimmed_line[..colon_pos].trim();
            let val_part = trimmed_line[colon_pos + 1..].trim();

            if !val_part.is_empty() && !val_part.starts_with('|') && !val_part.starts_with('>') {
                let candidate = val_part.trim_matches(|c| c == '"' || c == '\'');
                if let Some((decoded, is_bin, detected_type)) = try_decode_candidate(candidate) {
                    items.push(DecodedSecretItem {
                        key: Some(key_part.to_string()),
                        original: candidate.to_string(),
                        decoded: decoded.clone(),
                        is_binary: is_bin,
                        detected_type,
                    });

                    let prefix = &line[..indent];
                    let formatted_val = format_yaml_value(&decoded);
                    if formatted_val.starts_with("|\n") {
                        output_lines.push(format!("{prefix}{key_part}: {formatted_val}"));
                    } else {
                        output_lines.push(format!("{prefix}{key_part}: {formatted_val}"));
                    }
                    continue;
                }
            }
        }

        // 2. If not matched as YAML key-value, scan words / tokens in the line
        let mut modified_line = line.to_string();
        let mut line_replaced = false;

        // Split by whitespace and punctuation delimiters
        let words: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == ';' || c == '(' || c == ')')
            .filter(|w| w.len() >= 8)
            .collect();

        for word in words {
            let candidate = word.trim();
            if let Some((decoded, is_bin, detected_type)) = try_decode_candidate(candidate) {
                if !items.iter().any(|item| item.original == candidate) {
                    items.push(DecodedSecretItem {
                        key: None,
                        original: candidate.to_string(),
                        decoded: decoded.clone(),
                        is_binary: is_bin,
                        detected_type,
                    });
                }
                modified_line = modified_line.replace(candidate, &decoded);
                line_replaced = true;
            }
        }

        if line_replaced {
            output_lines.push(modified_line);
        } else {
            output_lines.push(line.to_string());
        }
    }

    let decoded_text = output_lines.join("\n");
    let count = items.len();

    AutoDecodeResult {
        ok: true,
        decoded_text,
        count,
        is_k8s_secret,
        items,
    }
}

/// Convert a YAML Secret with `stringData:` back to `data:` with Base64 encoded values,
/// or encode plaintext fields.
pub fn auto_encode_text(input: &str) -> AutoEncodeResult {
    let mut output_lines = Vec::new();
    let mut in_string_data_section = false;
    let mut string_data_indent = 0;
    let mut count = 0;
    let is_k8s_secret = input.contains("kind: Secret") || input.contains("stringData:");

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed_line = line.trim_start();
        let indent = line.len() - trimmed_line.len();

        // Check for stringData: section
        if trimmed_line.starts_with("stringData:") {
            in_string_data_section = true;
            string_data_indent = indent;
            let prefix = &line[..indent];
            output_lines.push(format!("{prefix}data:"));
            i += 1;
            continue;
        }

        // Check if we exited stringData: section
        if in_string_data_section && indent <= string_data_indent && !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
            in_string_data_section = false;
        }

        if in_string_data_section {
            // Check for key: value
            if let Some(colon_pos) = trimmed_line.find(':') {
                let key_part = trimmed_line[..colon_pos].trim();
                let val_part = trimmed_line[colon_pos + 1..].trim();

                // Check for multiline literal `|`
                if val_part == "|" || val_part == "|-" || val_part == "|+" {
                    let mut multiline_val = String::new();
                    let prefix = &line[..indent];
                    i += 1;
                    while i < lines.len() {
                        let next_line = lines[i];
                        let next_trimmed = next_line.trim_start();
                        let next_indent = next_line.len() - next_trimmed.len();
                        if next_indent > indent || next_trimmed.is_empty() {
                            multiline_val.push_str(next_trimmed);
                            multiline_val.push('\n');
                            i += 1;
                        } else {
                            break;
                        }
                    }
                    let encoded = b64::encode(multiline_val.trim_end().as_bytes(), false);
                    output_lines.push(format!("{prefix}{key_part}: {encoded}"));
                    count += 1;
                    continue;
                } else if !val_part.is_empty() {
                    // Single line value
                    let clean_val = unquote_yaml_value(val_part);
                    let encoded = b64::encode(clean_val.as_bytes(), false);
                    let prefix = &line[..indent];
                    output_lines.push(format!("{prefix}{key_part}: {encoded}"));
                    count += 1;
                    i += 1;
                    continue;
                }
            }
        }

        output_lines.push(line.to_string());
        i += 1;
    }

    if count == 0 && !is_k8s_secret {
        let encoded = b64::encode(input.as_bytes(), false);
        return AutoEncodeResult {
            ok: true,
            encoded_text: encoded,
            count: 1,
            is_k8s_secret: false,
        };
    }

    AutoEncodeResult {
        ok: true,
        encoded_text: output_lines.join("\n"),
        count,
        is_k8s_secret,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_decode_k8s_secret() {
        let yaml_input = r#"apiVersion: v1
kind: Secret
metadata:
  name: db-credentials
type: Opaque
data:
  connection_string: RGF0YSBTb3VyY2U9UENPUlBPUkFUSVZPLDMzNDE7SW5pdGlhbCBDYXRhbG9nPWRiX3JlbmFjaDtDb25uZWN0UmV0cnlDb3VudD0xMDA7UGVyc2lzdCBTZWN1cml0eSBJbmZvPVRydWU7VXNlciBJRD11c3JfYXBwX3BvcnRhbF9zaXN0ZW1hcztQYXNzd29yZD14aWlUOGZjTEZueW95OUxyQ3lTTTtNdWx0aXBsZUFjdGl2ZVJlc3VsdFNldHM9dHJ1ZTtUcnVzdFNlcnZlckNlcnRpZmljYXRlPVRydWU7
  user: dXNyX2FwcF9wb3J0YWxfc2lzdGVtYXM=
"#;

        let result = auto_decode_text(yaml_input, true);
        assert!(result.ok);
        assert_eq!(result.count, 2);
        assert!(result.is_k8s_secret);
        assert!(result.decoded_text.contains("stringData:"));
        assert!(result.decoded_text.contains("PCORPORATIVO,3341"));
        assert!(result.decoded_text.contains("usr_app_portal_sistemas"));
    }

    #[test]
    fn test_auto_encode_k8s_secret_roundtrip() {
        let decoded_yaml = r#"apiVersion: v1
kind: Secret
metadata:
  name: db-credentials
type: Opaque
stringData:
  connection_string: "Data Source=PCORPORATIVO,3341;Initial Catalog=db_renach;"
  user: "usr_app_portal_sistemas"
"#;

        let result = auto_encode_text(decoded_yaml);
        assert!(result.ok);
        assert!(result.is_k8s_secret);
        assert_eq!(result.count, 2);
        assert!(result.encoded_text.contains("data:"));
        assert!(result.encoded_text.contains("dXNyX2FwcF9wb3J0YWxfc2lzdGVtYXM="));

        // Roundtrip decode
        let re_decoded = auto_decode_text(&result.encoded_text, true);
        assert!(re_decoded.decoded_text.contains("PCORPORATIVO,3341"));
    }

    #[test]
    fn test_auto_decode_raw_string() {
        let raw = "RGF0YSBTb3VyY2U9UENPUlBPUkFUSVZPLDMzNDE7SW5pdGlhbCBDYXRhbG9nPWRiX3JlbmFjaDtDb25uZWN0UmV0cnlDb3VudD0xMDA7UGVyc2lzdCBTZWN1cml0eSBJbmZvPVRydWU7VXNlciBJRD11c3JfYXBwX3BvcnRhbF9zaXN0ZW1hcztQYXNzd29yZD14aWlUOGZjTEZueW95OUxyQ3lTTTtNdWx0aXBsZUFjdGl2ZVJlc3VsdFNldHM9dHJ1ZTtUcnVzdFNlcnZlckNlcnRpZmljYXRlPVRydWU7";
        let result = auto_decode_text(raw, false);
        assert!(result.ok);
        assert_eq!(result.count, 1);
        assert!(result.decoded_text.contains("PCORPORATIVO,3341"));
    }
}
