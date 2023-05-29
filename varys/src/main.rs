use speaker::Speaker;
use thiserror::Error;

#[derive(Error, Debug)]
enum Error {
    #[error(transparent)]
    Speaker(#[from] speaker::Error),
}

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let mut speaker = Speaker::new()?;

    speaker.set_voice("Isha")?;
    speaker.say("Tongue breaker".to_string(), false)?;
    speaker.set_voice("com.apple.voice.premium.en-GB.Malcolm")?;
    speaker.say("Fishermen's friends.".to_string(), false)?;

    Ok(())
}
