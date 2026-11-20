const VARIANTS: &[(&str, &str)] = &[
    ("acknowledgement", "acknowledgment"),
    ("acknowledgements", "acknowledgments"),
    ("analogue", "analog"),
    ("authorisation", "authorization"),
    ("authorised", "authorized"),
    ("behaviour", "behavior"),
    ("cancelled", "canceled"),
    ("centre", "center"),
    ("favour", "favor"),
    ("honour", "honor"),
    ("judgement", "judgment"),
    ("licence", "license"),
    ("licenced", "licensed"),
    ("licences", "licenses"),
    ("licencing", "licensing"),
    ("offence", "offense"),
    ("organisation", "organization"),
    ("organisations", "organizations"),
    ("practise", "practice"),
    ("recognised", "recognized"),
    ("sublicence", "sublicense"),
    ("utilise", "utilize"),
    ("whilst", "while"),
];

fn is_notice_line(line: &str) -> bool {
    let stripped = line.trim_start_matches(|c: char| c.is_whitespace() || "#*/-=>|;".contains(c)).to_lowercase();
    if stripped.starts_with("(c)") || stripped.starts_with('\u{a9}') || stripped.trim_end_matches(['.', ' ']) == "all rights reserved" {
        return true;
    }
    let Some(rest) = stripped.strip_prefix("copyright") else { return false };
    let next = rest.trim_start_matches([':', ' ', '\t']).split_whitespace().next().unwrap_or("");
    let next = next.trim_end_matches(|c: char| !c.is_alphanumeric());
    !SENTENCE_WORDS.contains(&next)
}

const SENTENCE_WORDS: &[&str] = &[
    "act", "and", "are", "as", "holder", "holders", "in", "interest", "is", "law", "laws", "license", "licenses", "notice", "notices",
    "of", "or", "owner", "owners", "permission", "protection", "rights", "statement", "statements", "the", "to", "which",
];

fn clean_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for token in line.split_whitespace() {
        let lower = token.to_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.") || lower.starts_with("<http") {
            continue;
        }
        out.push_str(&lower);
        out.push(' ');
    }
    out.replace('&', " and ").replace(['-', '\'', '\u{2019}', '\u{2010}', '\u{2011}'], "")
}

fn sequence(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines().filter(|line| !is_notice_line(line)) {
        for word in clean_line(line).split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()) {
            out.push(VARIANTS.iter().find(|(from, _)| *from == word).map(|(_, to)| *to).unwrap_or(word).to_string());
        }
    }
    out
}

pub fn words(text: &str) -> Vec<String> {
    let set: std::collections::BTreeSet<String> = sequence(text).into_iter().collect();
    set.into_iter().collect()
}

#[allow(dead_code)]
pub fn phrase(text: &str) -> String {
    format!(" {} ", sequence(text).join(" "))
}
