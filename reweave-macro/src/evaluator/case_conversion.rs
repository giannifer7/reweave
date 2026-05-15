use std::str::FromStr;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Lower,          // lowercase
    Upper,          // UPPERCASE
    Snake,          // snake_case
    Screaming,      // SCREAMING_SNAKE_CASE
    Kebab,          // kebab-case
    ScreamingKebab, // SCREAMING-KEBAB-CASE
    Camel,          // camelCase
    Pascal,         // PascalCase
    Ada,            // Ada_Case
}

impl FromStr for Case {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "lower" | "lowercase" => Ok(Case::Lower),
            "upper" | "uppercase" => Ok(Case::Upper),
            "snake" | "snake_case" => Ok(Case::Snake),
            "screaming" | "screaming_snake" | "screaming_snake_case" => Ok(Case::Screaming),
            "kebab" | "kebab-case" | "kebab_case" => Ok(Case::Kebab),
            "screaming-kebab"
            | "screaming-kebab-case"
            | "screaming_kebab"
            | "screaming_kebab_case" => Ok(Case::ScreamingKebab),
            "camel" | "camelcase" | "camel_case" => Ok(Case::Camel),
            "pascal" | "pascalcase" | "pascal_case" => Ok(Case::Pascal),
            "ada" | "ada_case" => Ok(Case::Ada),
            _ => Err(format!("Unknown case style: {}", s)),
        }
    }
}
#[derive(Debug)]
struct WordSplitter<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> WordSplitter<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn is_boundary_char(c: char) -> bool {
        c == '_' || c == '-' || c.is_whitespace()
    }

    fn is_word_boundary(prev: Option<char>, curr: char, next: Option<char>) -> bool {
        if Self::is_boundary_char(curr) {
            return true;
        }

        match (prev, curr, next) {
            // Start of an acronym (XMLHttpRequest -> XML|Http|Request)
            (Some(p), c, Some(n)) if p.is_uppercase() && c.is_uppercase() && n.is_lowercase() => {
                true
            }

            // Transition from lowercase to uppercase (camelCase -> camel|Case)
            (Some(p), c, _) if p.is_lowercase() && c.is_uppercase() => true,

            // Transition between letter and number
            (Some(p), c, _) if p.is_ascii_alphabetic() && c.is_ascii_digit() => true,
            (Some(p), c, _) if p.is_ascii_digit() && c.is_ascii_alphabetic() => true,

            _ => false,
        }
    }
}

impl<'a> Iterator for WordSplitter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        // Skip leading delimiters
        while self.pos < self.input.len()
            && Self::is_boundary_char(self.input[self.pos..].chars().next().unwrap())
        {
            self.pos += 1;
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let start = self.pos;
        let mut chars = self.input[self.pos..].char_indices();
        let mut last_pos = 0;
        let mut prev_char = None;

        while let Some((i, curr_char)) = chars.next() {
            let next_char = chars.clone().next().map(|(_, c)| c);

            if Self::is_word_boundary(prev_char, curr_char, next_char) && i > 0 {
                self.pos += i;
                return Some(&self.input[start..start + i]);
            }

            last_pos = i;
            prev_char = Some(curr_char);
        }

        // Handle the last word
        self.pos = self.input.len();
        Some(&self.input[start..start + last_pos + 1])
    }
}
pub fn convert_case_str(input: &str, target_case: &str) -> Result<String, String> {
    let case = target_case.parse::<Case>()?;
    Ok(convert_case(input, case))
}

pub fn convert_case(input: &str, target_case: Case) -> String {
    let words: Vec<&str> = WordSplitter::new(input).collect();

    if words.is_empty() {
        return String::new();
    }

    match target_case {
        Case::Lower => words.join("").to_lowercase(),

        Case::Upper => words.join("").to_uppercase(),

        Case::Snake => words
            .into_iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("_"),

        Case::Screaming => words
            .into_iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),

        Case::Kebab => words
            .into_iter()
            .map(|w| w.to_lowercase())
            .collect::<Vec<_>>()
            .join("-"),

        Case::ScreamingKebab => words
            .into_iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("-"),

        Case::Camel => {
            let mut result = String::new();
            for (i, word) in words.into_iter().enumerate() {
                if i == 0 {
                    result.push_str(&word.to_lowercase());
                } else {
                    result.push_str(&capitalize(word));
                }
            }
            result
        }

        Case::Pascal => words
            .into_iter()
            .map(capitalize)
            .collect::<Vec<_>>()
            .join(""),

        Case::Ada => words
            .into_iter()
            .map(capitalize)
            .collect::<Vec<_>>()
            .join("_"),
    }
}
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = first.to_uppercase().collect::<String>();
            result.extend(chars.flat_map(|c| c.to_lowercase()));
            result
        }
    }
}
