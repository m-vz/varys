use varys::cli;
use varys::error::Error;

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    cli::run()
}
