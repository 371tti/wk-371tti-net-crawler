/// URL percent-decode and application/x-www-form-urlencoded style decoding.
///
/// This module provides a simple, dependency-free decoder that:
/// - converts `+` to space
/// - decodes `%HH` hex sequences into bytes
/// - returns a UTF-8 String using lossy conversion when bytes are invalid UTF-8
///
/// Use `url_decode` for a forgiving decode and `url_decode_strict` if you
/// want an error on malformed percent-escapes.
#[cfg(not(feature = "lib"))]
pub fn url_decode(input: &str) -> String {
	match url_decode_strict(input) {
		Ok(s) => s,
		Err(_) => {
			// Fallback to lossy decode: replace invalid bytes with replacement char
			let mut out: Vec<u8> = Vec::with_capacity(input.len());
			let bytes = input.as_bytes();
			let mut i = 0;
			while i < bytes.len() {
				match bytes[i] {
					b'+' => { out.push(b' '); i += 1; }
					b'%' => {
						if i + 2 < bytes.len() {
							let hi = from_hex(bytes[i+1]);
							let lo = from_hex(bytes[i+2]);
							if hi.is_some() && lo.is_some() {
								out.push((hi.unwrap() << 4) | lo.unwrap());
								i += 3;
							} else {
								// invalid hex -> keep '%' as-is
								out.push(b'%');
								i += 1;
							}
						} else {
							out.push(b'%');
							i += 1;
						}
					}
					b => { out.push(b); i += 1; }
				}
			}
			String::from_utf8_lossy(&out).into_owned()
		}
	}
}

/// Strict decoder: returns Err when encountering malformed `%` sequences.
#[cfg(not(feature = "lib"))]
pub fn url_decode_strict(input: &str) -> Result<String, String> {
	let mut out: Vec<u8> = Vec::with_capacity(input.len());
	let bytes = input.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		match bytes[i] {
			b'+' => { out.push(b' '); i += 1; }
			b'%' => {
				if i + 2 >= bytes.len() {
					return Err(format!("incomplete percent-escape at pos {}", i));
				}
				let hi = from_hex(bytes[i+1]);
				let lo = from_hex(bytes[i+2]);
				if hi.is_none() || lo.is_none() {
					return Err(format!("invalid percent-escape at pos {}", i));
				}
				out.push((hi.unwrap() << 4) | lo.unwrap());
				i += 3;
			}
			b => { out.push(b); i += 1; }
		}
	}
	match String::from_utf8(out) {
		Ok(s) => Ok(s),
		Err(e) => Err(format!("invalid utf8 after decoding: {}", e)),
	}
}

#[cfg(not(feature = "lib"))]
fn from_hex(b: u8) -> Option<u8> {
	match b {
		b'0'..=b'9' => Some(b - b'0'),
		b'a'..=b'f' => Some(b - b'a' + 10),
		b'A'..=b'F' => Some(b - b'A' + 10),
		_ => None,
	}
}

#[cfg(not(feature = "lib"))]
pub fn url_normalize(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else {
        let base = base_url.trim_end_matches('/');
        if href.starts_with('/') {
            format!("{}/{}", base, href.trim_start_matches('/'))
        } else {
            format!("{}/{}", base, href)
        }
    }
}

#[cfg(not(feature = "lib"))]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decode_simple() {
		assert_eq!(url_decode("hello%20world"), "hello world");
		assert_eq!(url_decode("a+b+c"), "a b c");
	}

	#[test]
	fn decode_utf8() {
		// %E3%81%82 == あ (UTF-8)
		assert_eq!(url_decode("%E3%81%82"), "あ");
	}

	#[test]
	fn strict_errors() {
		assert!(url_decode_strict("%E3%81").is_err()); // incomplete
		assert!(url_decode_strict("%ZZ").is_err()); // invalid hex
	}

	#[test]
	fn fallback_lossy() {
		// malformed percent -> leaves '%' when strict fails
		assert_eq!(url_decode("%ZZ"), "%ZZ");
	}
}

