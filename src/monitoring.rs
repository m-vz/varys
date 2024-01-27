use log::info;
use reqwest::Url;

use crate::error::Error;

/// Ping a passive monitoring service with a specific message.
///
/// The url to the service must be set in the `VARYS_MONITORING_URL` environment variable. All instances
/// if the string `{varys_message}` will be replaced with the message.
///
/// # Arguments
///
/// * `message`: What message to send to the monitoring service.
///
/// Returns an error if the request failed.
pub async fn ping(message: &str) -> Result<(), Error> {
    let url = dotenvy::var("VARYS_MONITORING_URL").map_err(|_| Error::MissingMonitoringUrl)?;
    let url = url.replace("{varys_message}", message);
    let url = Url::parse(&url).map_err(|_| Error::InvalidMonitoringUrl(url))?;

    info!(
        "Pinging monitoring at {} with message: {message}",
        url.domain().unwrap_or(url.as_str())
    );

    reqwest::get(url)
        .await
        .map_err(Error::MonitoringConnectionFailed)?;

    Ok(())
}
