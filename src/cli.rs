use colored::{ColoredString, Colorize};
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Unable to read user input.")]
    UserInput,
}

#[derive(PartialEq)]
pub enum KeyType {
    Key(char),
    Enter,
    Illegal,
}

impl KeyType {
    fn join(keys: &[KeyType], separator: &str) -> String {
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

pub fn user_choice(text: &str, choices: &[KeyType]) -> Result<KeyType, Error> {
    let mut writer = io::BufWriter::new(io::stdout());
    write!(writer, "{} {}", text, action_description(choices)).map_err(|_| Error::UserInput)?;
    writer.flush().map_err(|_| Error::UserInput)?;
    let result = read_single_char().map(KeyType::from);
    writeln!(writer).map_err(|_| Error::UserInput)?;
    result
}

pub fn user_confirmation(text: &str) -> Result<(), Error> {
    user_choice(text, &[KeyType::Enter]).map(|_| ())
}

fn read_single_char() -> Result<char, Error> {
    crossterm::terminal::enable_raw_mode().map_err(|_| Error::UserInput)?;

    let mut input = [0_u8];
    io::stdin()
        .read_exact(&mut input)
        .map_err(|_| Error::UserInput)?;

    crossterm::terminal::disable_raw_mode().map_err(|_| Error::UserInput)?;

    Ok(input[0] as char)
}

fn action_description(actions: &[KeyType]) -> ColoredString {
    format!("({})", KeyType::join(actions, " / ")).bright_black()
}
