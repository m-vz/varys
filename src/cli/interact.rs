use crate::cli::key_type::KeyType;
use colored::Colorize;
use std::io;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    InputOutput(#[from] io::Error),
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
    let choices_description = format!("({})", KeyType::join(choices, " / ")).bright_black();
    write!(writer, "{} {}", text, choices_description)?;
    writer.flush()?;

    loop {
        let key = read_single_char().map(KeyType::from)?;
        writeln!(writer)?;

        if choices.contains(&key) {
            return Ok(key);
        } else {
            write!(
                writer,
                "Press {}{} to continue...",
                if choices.len() > 1 { "one of " } else { "" },
                choices_description
            )?;
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
