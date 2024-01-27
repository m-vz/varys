use std::fmt::{Display, Formatter, Write};

/// Represents a single key on the keyboard.
///
/// Can be parsed from a `char` taken from user input.
#[derive(PartialEq)]
pub enum KeyType {
    Key(char),
    Enter,
    CtrlC,
    Illegal,
}

impl KeyType {
    /// Join keys into a string, separated by a separator.
    ///
    /// # Arguments
    ///
    /// * `keys`: The keys to join together.
    /// * `separator`: The separator to separate the keys with.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::cli::key_type::KeyType;
    /// assert_eq!(KeyType::join(&[KeyType::from('r'), KeyType::Enter], ", "), "r, Enter");
    /// ```
    pub fn join(keys: &[KeyType], separator: &str) -> String {
        let full = keys.iter().fold(String::new(), |mut output, k| {
            let _ = write!(output, "{}{}", k, separator);
            output
        });
        full[..full.len() - separator.len()].to_string()
    }
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                KeyType::Key(key) => key.to_string(),
                KeyType::Enter => "Enter".to_string(),
                KeyType::CtrlC => "Ctrl+C".to_string(),
                KeyType::Illegal => "ILLEGAL".to_string(),
            }
        )
    }
}

impl From<char> for KeyType {
    fn from(value: char) -> Self {
        match value {
            'a'..='z' | 'A'..='Z' | '0'..='9' => KeyType::Key(value),
            '\n' | '\r' => KeyType::Enter,
            '\u{3}' => KeyType::CtrlC,
            _ => KeyType::Illegal,
        }
    }
}
