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
    speaker.say("YabbaDabbaDoooo".to_string(), false)?;
    for x in 1..=3 {
        speaker.say(x.to_string(), false)?;
    }

    speaker::start_run_loop();

    Ok(())
}
