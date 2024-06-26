use std::fs;
use std::fs::{DirEntry, File};
use std::io::Write;
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
    dataset.save(&data_dir)?;
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

pub fn test_dataset<P: AsRef<Path>>(data_dir: P) -> Result<(), Error> {
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

pub fn test_single<P: AsRef<Path>>(
    data_dir: P,
    capture_path: P,
    address: &MacAddress,
) -> Result<Vec<(String, f32)>, Error> {
    let device = WgpuDevice::default();
    let trace = NumericTraceDataset::load_trace(capture_path, address)?;
    let (_, _, testing_dataset) = NumericTraceDataset::load(&data_dir)?.split_default()?;
    let output = inference::infer::<AutodiffBackend>(
        data_dir.as_ref().to_string_lossy().as_ref(),
        trace,
        device,
    )?
    .flatten::<1>(0, 1)
    .to_data()
    .value;

    Ok(testing_dataset.queries.into_iter().zip(output).collect())
}

pub fn infer<P: AsRef<Path>>(
    data_dir: P,
    item: &NumericTraceItem,
    testing_dataset: &NumericTraceDataset,
    device: &WgpuDevice,
) -> Result<u8, Error> {
    let recognised = inference::predict(
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

pub fn compile_all_logs<P: AsRef<Path>>(data_dir: P, id: &str) -> Result<(), Error> {
    compile_logs(&data_dir, "train", id)?;
    compile_logs(&data_dir, "valid", id)
}

fn compile_logs<P: AsRef<Path>>(data_dir: P, name: &str, id: &str) -> Result<(), Error> {
    let log_dir = data_dir.as_ref().join("ml").join(name);
    let mut csv = File::create(
        data_dir
            .as_ref()
            .join("ml")
            .join(format!("{id}-{name}.csv")),
    )?;
    let mut epochs = fs::read_dir(log_dir)?
        .filter_map(|dir| dir.ok())
        .filter(|dir| {
            dir.file_name()
                .into_string()
                .is_ok_and(|path| path.contains("epoch"))
        })
        .collect::<Vec<_>>();
    epochs.sort_by_key(epoch_number);

    for epoch in epochs {
        let number = epoch_number(&epoch);
        let accuracy = fs::read_to_string(epoch.path().join("Accuracy.log"))?;
        let accuracy_sum: f64 = accuracy
            .lines()
            .filter_map(|line| line.parse::<f64>().ok())
            .sum();
        let accuracy_average = accuracy_sum / accuracy.lines().count() as f64;
        let loss = fs::read_to_string(epoch.path().join("Loss.log"))?;
        let loss_sum: f64 = loss
            .lines()
            .filter_map(|line| line.parse::<f64>().ok())
            .sum();
        let loss_average = loss_sum / loss.lines().count() as f64;
        writeln!(csv, "{number},{accuracy_average},{loss_average}")?;
    }

    Ok(())
}

fn epoch_number(epoch: &DirEntry) -> usize {
    epoch
        .file_name()
        .into_string()
        .unwrap_or_default()
        .split('-')
        .last()
        .unwrap_or_default()
        .parse::<usize>()
        .unwrap_or_default()
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
