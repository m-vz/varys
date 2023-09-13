use varys::cli;
use varys::error::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().map_err(|error| Error::Dotenv(error.to_string()))?;
    pretty_env_logger::init();

    cli::run().await
}
