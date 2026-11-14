pub fn parse(text: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&strip(text)).ok()
}

fn strip(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    let mut in_string = false;
    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();
        if in_string {
            out.push(current);
            if current == '\\' {
                if let Some(escaped) = next {
                    out.push(escaped);
                    index += 1;
                }
            } else if current == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        if current == '"' {
            in_string = true;
            out.push(current);
        } else if current == '/' && next == Some('/') {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        } else if current == '/' && next == Some('*') {
            index += 2;
            while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            index += 2;
            continue;
        } else if current == ',' && closes_after(&chars, index + 1) {
            index += 1;
            continue;
        } else {
            out.push(current);
        }
        index += 1;
    }
    out
}

fn closes_after(chars: &[char], from: usize) -> bool {
    chars[from..].iter().find(|character| !character.is_whitespace()).map_or(false, |next| *next == '}' || *next == ']')
}

#[cfg(test)]
mod tests {
    #[test]
    fn strips_comments_and_trailing_commas() {
        let value = super::parse("{ // note\n \"a\": \"x//y\", /* c */ \"b\": [1, 2,], }").unwrap();
        assert_eq!(value["a"], "x//y");
        assert_eq!(value["b"][1], 2);
    }
}
