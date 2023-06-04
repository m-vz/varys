use colored::{ColoredString, Colorize};
use std::fmt::{Display, Formatter};
use std::io;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] io::Error),
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

pub fn user_input(
    text: &str,
    mut validation: impl FnMut(&str) -> bool,
    invalid_message: &str,
) -> Result<String, Error> {
    let mut writer = io::BufWriter::new(io::stdout());
    write!(writer, "{} ", text)?;
    writer.flush()?;

    let mut input = String::new();
    loop {
        io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();
        if validation(&input) {
            return Ok(input);
        } else {
            write!(writer, "{} ", invalid_message)?;
            writer.flush()?;
        }
    }
}

pub fn user_choice(text: &str, choices: &[KeyType]) -> Result<KeyType, Error> {
    let mut writer = io::BufWriter::new(io::stdout());
    write!(writer, "{} {}", text, action_description(choices))?;
    writer.flush()?;

    loop {
        let key = read_single_char().map(KeyType::from)?;
        writeln!(writer)?;

        if choices.contains(&key) {
            return Ok(key);
        } else {
            if choices.len() > 1 {
                write!(
                    writer,
                    "Press one of {} to continue...",
                    action_description(choices)
                )?;
            } else {
                write!(
                    writer,
                    "Press {} to continue...",
                    action_description(choices)
                )?;
            }
            writer.flush()?;
        }
    }
}

pub fn user_confirmation(text: &str) -> Result<(), Error> {
    user_choice(text, &[KeyType::Enter]).map(|_| ())
}

fn read_single_char() -> Result<char, Error> {
    crossterm::terminal::enable_raw_mode()?;

    let mut input = [0_u8];
    io::stdin().read_exact(&mut input)?;

    crossterm::terminal::disable_raw_mode()?;

    Ok(input[0] as char)
}

fn action_description(actions: &[KeyType]) -> ColoredString {
    format!("({})", KeyType::join(actions, " / ")).bright_black()
}
