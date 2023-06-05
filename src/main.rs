use varys::cli;
use varys::Error;

fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    cli::run()
}
