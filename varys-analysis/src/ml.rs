use std::fs;
use std::path::{Path, PathBuf};

use burn::backend::wgpu::{AutoGraphicsApi, WgpuDevice};
use burn::backend::{Autodiff, Wgpu};
use burn::data::dataset::Dataset;
use burn::optim::AdamConfig;
use log::info;

use cnn::training;
use varys_database::database::interaction::Interaction;
use varys_network::address::MacAddress;

use crate::error::Error;
use crate::ml::cnn::training::CNNTrainingConfig;
use crate::ml::cnn::{inference, CNNModelConfig};
use crate::ml::data::{NumericTraceItem, SplitNumericTraceDataset};

mod activation;
mod cnn;
pub mod data;

type Backend = Wgpu<AutoGraphicsApi, f32, i32>;
type AutodiffBackend = Autodiff<Backend>;

pub fn train<P: AsRef<Path>>(
    data_dir: P,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    let data_dir_string = data_dir.as_ref().to_string_lossy().to_string();
    fs::create_dir_all(ml_path(&data_dir_string))?;

    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::load_or_create(data_dir, interactions, relative_to)?;

    info!("Beginning training...");

    training::train::<AutodiffBackend>(
        &data_dir_string,
        CNNTrainingConfig::new(
            CNNModelConfig::new(dataset.full.num_labels()),
            AdamConfig::new(),
        ),
        dataset,
        device,
    )?;

    println!("Training complete");

    Ok(())
}

pub fn test<P: AsRef<Path>>(
    data_dir: P,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::load_or_create(&data_dir, interactions, relative_to)?;

    for index in 0..dataset.testing.len() {
        if let Some(item) = &dataset.testing.get(index) {
            infer(&data_dir, item, &dataset, &device)?;
        }
    }

    Ok(())
}

pub fn infer<P: AsRef<Path>>(
    data_dir: P,
    item: &NumericTraceItem,
    dataset: &SplitNumericTraceDataset,
    device: &WgpuDevice,
) -> Result<u8, Error> {
    let recognised = inference::infer::<AutodiffBackend>(
        data_dir.as_ref().to_string_lossy().as_ref(),
        item.trace.clone(),
        device.clone(),
    )?;

    println!(
        "Recognised \"{}\" as \"{}\"",
        dataset.full.get_query(item.label).unwrap_or_default(),
        dataset.full.get_query(recognised).unwrap_or_default(),
    );

    Ok(recognised)
}

fn dataset_path<P: AsRef<Path>>(data_dir: P) -> PathBuf {
    PathBuf::from(format!(
        "{}/dataset.json",
        ml_path(data_dir.as_ref().to_string_lossy().as_ref())
    ))
}

fn model_path(data_dir: &str) -> String {
    format!("{}/model", ml_path(data_dir))
}

fn config_path(data_dir: &str) -> String {
    format!("{}/config.json", ml_path(data_dir))
}

fn ml_path(data_dir: &str) -> String {
    format!("{data_dir}/ml")
}
