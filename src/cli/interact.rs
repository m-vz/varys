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

/// Validated input from the user. Only supports single-line input.
///
/// This will block until the user has entered a valid input.
///
/// # Arguments
///
/// * `text`: The text displayed before the initial input.
/// * `validation`: A function testing whether the input is valid.
/// * `invalid_message`: The message shown if the user enters invalid input.
///
/// # Examples
///
/// ```no_run
/// # use varys::cli::interact::user_input;
/// user_input(
///     "Enter a number between 0 and 255:",
///     |i| i.parse::<u8>().is_ok(),
///     "Wrong input, try again:"
/// ).unwrap();
/// ```
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

/// Let the user choose between multiple options by pressing a specific key.
///
/// This will block until the user has pressed a valid key.
///
/// # Arguments
///
/// * `text`: The text displayed before the initial input.
/// * `choices`: A list of keys the user can press.
///
/// Returns the pressed key.
///
/// # Examples
///
/// ```no_run
/// # use varys::cli::interact::user_choice;
/// # use varys::cli::key_type::KeyType;
/// user_choice("Confirm or repeat", &[KeyType::Enter, KeyType::Key('r')]).unwrap();
/// ```
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

/// Ask the user for confirmation before continuing.
///
/// This will block until the user has pressed Enter.
///
/// # Arguments
///
/// * `text`: The text displayed to the user before waiting.
///
/// # Examples
///
/// ```no_run
/// # use varys::cli::interact::user_confirmation;
/// user_confirmation("Confirm to continue").unwrap();
/// ```
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
