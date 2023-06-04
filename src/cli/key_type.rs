use std::fmt::{Display, Formatter};

#[derive(PartialEq)]
pub enum KeyType {
    Key(char),
    Enter,
    Illegal,
}

impl KeyType {
    pub fn join(keys: &[KeyType], separator: &str) -> String {
        let full: String = keys.iter().map(|k| format!("{}{}", k, separator)).collect();
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
            _ => KeyType::Illegal,
        }
    }
}
