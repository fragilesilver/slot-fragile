// Ported from nickclyde/slot-plus@02c5890, crates/slot-store/src/find.rs.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folded {
    pub words: Vec<String>,
    pub joined: String,
}

impl Folded {
    pub fn is_empty(&self) -> bool {
        self.joined.is_empty()
    }
}

pub fn fold(text: &str) -> Folded {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut depth = 0u32;
    for c in text.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                end_word(&mut word, &mut words);
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                end_word(&mut word, &mut words);
            }
            _ if depth > 0 => {}
            '\'' | '\u{2019}' => {}
            _ => match plain(c) {
                Some(p) => word.push_str(p),
                None if c.is_alphanumeric() => word.extend(c.to_lowercase()),
                None => end_word(&mut word, &mut words),
            },
        }
    }
    end_word(&mut word, &mut words);
    let joined = words.concat();
    Folded { words, joined }
}

fn end_word(word: &mut String, words: &mut Vec<String>) {
    if !word.is_empty() {
        words.push(std::mem::take(word));
    }
}

fn plain(c: char) -> Option<&'static str> {
    Some(match c {
        'a'..='z' | '0'..='9' => return None,
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
        'ý' | 'ÿ' | 'Ý' => "y",
        'æ' | 'Æ' => "ae",
        'œ' | 'Œ' => "oe",
        'ß' => "ss",
        _ => return None,
    })
}

const INSIDE_MIN: usize = 3;

pub fn rank(title: &Folded, query: &Folded) -> Option<u8> {
    if query.is_empty() {
        return None;
    }
    if title.joined.starts_with(&query.joined) {
        return Some(0);
    }
    let starts = |q: &String| title.words.iter().any(|w| w.starts_with(q.as_str()));
    if query.words.iter().all(starts) {
        return Some(1);
    }
    let inside =
        |q: &String| starts(q) || (q.len() >= INSIDE_MIN && title.joined.contains(q.as_str()));
    query.words.iter().all(inside).then_some(2)
}

pub fn matches(titles: &[Folded], query: &Folded) -> Vec<usize> {
    let mut hits: Vec<(u8, usize)> = titles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| rank(t, query).map(|r| (r, i)))
        .collect();
    hits.sort();
    hits.into_iter().map(|(_, i)| i).collect()
}
