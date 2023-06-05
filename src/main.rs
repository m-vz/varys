use log::{debug, info};
use pcap::ConnectionStatus;
use std::path::PathBuf;
use varys::sniff::Sniffer;
use varys::Error;
use varys::{cli, sniff};

const PCAP_PATH: &str = "data/captures/captured.pcap";

fn main() -> Result<(), Error> {
    pretty_env_logger::init();
    cli::run()
}

fn sniff() -> Result<(), Error> {
    info!("Sniffing...");
    for device in sniff::devices_with_status(&ConnectionStatus::Connected)? {
        debug!("{}", Sniffer::from(device));
    }
    let sniffer = Sniffer::from(sniff::device_by_name("ap1")?);
    debug!("Using: {}", sniffer);
    let stats = sniffer.run_for(5, Some(PathBuf::from(PCAP_PATH)))?;
    debug!("Stats: {}", stats);

    Ok(())
}
