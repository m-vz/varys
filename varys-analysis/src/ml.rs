use std::fs;

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
mod data;

type Backend = Wgpu<AutoGraphicsApi, f32, i32>;
type AutodiffBackend = Autodiff<Backend>;

pub fn train(
    data_dir: &str,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    fs::create_dir_all(data_dir)?;

    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::load_or_create(data_dir, interactions, relative_to)?;

    info!("Beginning training...");

    training::train::<AutodiffBackend>(
        data_dir,
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

pub fn test(
    data_dir: &str,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::load_or_create(data_dir, interactions, relative_to)?;

    for index in 0..dataset.testing.len() {
        if let Some(item) = &dataset.testing.get(index) {
            infer(data_dir, item, &dataset, &device)?;
        }
    }

    Ok(())
}

pub fn infer(
    data_dir: &str,
    item: &NumericTraceItem,
    dataset: &SplitNumericTraceDataset,
    device: &WgpuDevice,
) -> Result<u8, Error> {
    let recognised =
        inference::infer::<AutodiffBackend>(data_dir, item.trace.clone(), device.clone())?;

    println!(
        "Recognised \"{}\" as \"{}\"",
        dataset.full.get_query(item.label).unwrap_or_default(),
        dataset.full.get_query(recognised).unwrap_or_default(),
    );

    Ok(recognised)
}

fn model_path(data_dir: &str) -> String {
    format!("{data_dir}/model")
}

fn config_path(data_dir: &str) -> String {
    format!("{data_dir}/config.json")
}
