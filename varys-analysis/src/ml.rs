use std::fs;
use std::path::{Path, PathBuf};

use burn::backend::wgpu::{AutoGraphicsApi, WgpuDevice};
use burn::backend::{Autodiff, Wgpu};
use burn::data::dataset::Dataset;
use burn::optim::AdamConfig;
use log::info;

use cnn::training;
use varys_database::database::interaction::Interaction;

use crate::error::Error;
use crate::ml::cnn::training::CNNTrainingConfig;
use crate::ml::cnn::{inference, CNNModelConfig};
use crate::ml::data::{NumericTraceDataset, NumericTraceItem};

mod activation;
mod cnn;
pub mod data;

type Backend = Wgpu<AutoGraphicsApi, f32, i32>;
type AutodiffBackend = Autodiff<Backend>;

pub fn train<P: AsRef<Path>>(data_dir: P, interactions: Vec<Interaction>) -> Result<(), Error> {
    let data_dir_string = data_dir.as_ref().to_string_lossy().to_string();
    fs::create_dir_all(ml_path(&data_dir_string))?;

    let device = WgpuDevice::default();
    let mut dataset = NumericTraceDataset::load_or_new(&data_dir, interactions)?;
    dataset
        .normalise()
        .resize_all(CNNModelConfig::DEFAULT_INPUT_DIMENSIONS)
        .shuffle();
    dataset.save(dataset_path(&data_dir))?;
    let config = CNNTrainingConfig::new(
        CNNModelConfig::new(
            dataset.num_labels(),
            CNNModelConfig::DEFAULT_INPUT_DIMENSIONS,
        ),
        AdamConfig::new(),
    );
    let (training_dataset, validation_dataset, _) = dataset.split_default()?;

    info!("Beginning training...");

    training::train::<AutodiffBackend>(
        &data_dir_string,
        config,
        training_dataset,
        validation_dataset,
        device,
    )?;

    println!("Training complete");

    Ok(())
}

pub fn test<P: AsRef<Path>>(data_dir: P) -> Result<(), Error> {
    let device = WgpuDevice::default();
    let (_, _, testing_dataset) = NumericTraceDataset::load(&data_dir)?.split_default()?;
    let mut num_correct = 0;

    for index in 0..testing_dataset.len() {
        if let Some(item) = &testing_dataset.get(index) {
            if infer(&data_dir, item, &testing_dataset, &device)? == item.label {
                num_correct += 1;
            }

            println!(
                "Recognised {num_correct}/{} correctly ({:.2}%)",
                index + 1,
                num_correct as f32 * 100. / (index + 1) as f32
            );
        }
    }

    Ok(())
}

pub fn infer<P: AsRef<Path>>(
    data_dir: P,
    item: &NumericTraceItem,
    testing_dataset: &NumericTraceDataset,
    device: &WgpuDevice,
) -> Result<u8, Error> {
    let recognised = inference::infer::<AutodiffBackend>(
        data_dir.as_ref().to_string_lossy().as_ref(),
        item.trace.clone(),
        device.clone(),
    )?;

    println!(
        "Recognised \"{}\"\nas         \"{}\"",
        testing_dataset.get_query(item.label).unwrap_or_default(),
        testing_dataset.get_query(recognised).unwrap_or_default(),
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
