use colored::{ColoredString, Colorize};
use std::io;
use std::io::Write;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Unable to read user input.")]
    UserInput,
}

pub fn user_confirmation(text: &str) -> Result<(), Error> {
    write!(
        stdout_writer(),
        "{} {}",
        text,
        action_description(vec!["Enter"])
    )
    .map_err(|_| Error::UserInput)?;
    stdout_writer().flush().map_err(|_| Error::UserInput)?;
    io::stdin()
        .read_line(&mut String::new())
        .map_err(|_| Error::UserInput)
        .map(|_| ())
}

fn stdout_writer() -> io::BufWriter<io::Stdout> {
    io::BufWriter::new(io::stdout())
}

fn action_description(actions: Vec<&str>) -> ColoredString {
    format!("({})", actions.join(" / ")).bright_black()
}
