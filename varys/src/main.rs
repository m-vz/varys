use log::error;

use varys::cli;
use varys::error::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    main_fallible().await.map_err(|error| {
        error!("{error}");

        error
    })
}

async fn main_fallible() -> Result<(), Error> {
    dotenvy::dotenv().map_err(|error| Error::Dotenv(error.to_string()))?;
    pretty_env_logger::init();

    cli::run().await
}
