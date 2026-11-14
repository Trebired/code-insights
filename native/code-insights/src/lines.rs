use crate::output::Lines;

pub fn count(text: &str) -> Lines {
    let mut total = 0u64;
    let mut content = 0u64;
    for line in text.lines() {
        total += 1;
        if !line.trim().is_empty() {
            content += 1;
        }
    }
    Lines { total, content, code: content }
}

pub fn is_binary(bytes: &[u8]) -> bool {
    let probe = &bytes[..bytes.len().min(8000)];
    probe.contains(&0) || std::str::from_utf8(bytes).is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_total_and_content_lines() {
        let lines = count("a\n\n  \nb\n");
        assert_eq!((lines.total, lines.content, lines.code), (4, 2, 2));
    }

    #[test]
    fn detects_binary() {
        assert!(is_binary(&[0x61, 0x00, 0x62]));
        assert!(is_binary(&[0xff, 0xfe, 0xfd]));
        assert!(!is_binary("hello".as_bytes()));
    }
}
