use std::fs;

use burn::backend::wgpu::{AutoGraphicsApi, WgpuDevice};
use burn::backend::{Autodiff, Wgpu};
use burn::optim::AdamConfig;
use log::info;

use cnn::training;
use varys_database::database::interaction::Interaction;
use varys_network::address::MacAddress;

use crate::error::Error;
use crate::ml::cnn::training::CNNTrainingConfig;
use crate::ml::cnn::{inference, CNNModelConfig};
use crate::ml::data::{NumericTraceDataset, SplitNumericTraceDataset};

mod activation;
mod cnn;
mod data;

type Backend = Wgpu<AutoGraphicsApi, f32, i32>;
type AutodiffBackend = Autodiff<Backend>;

const TRAINING_PROPORTION: f64 = 0.64;
const VALIDATION_PROPORTION: f64 = 0.16;
const TESTING_PROPORTION: f64 = 0.2;

pub fn train(
    data_dir: &str,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::split(
        NumericTraceDataset::load(interactions, relative_to)?,
        TRAINING_PROPORTION,
        VALIDATION_PROPORTION,
        TESTING_PROPORTION,
    )?;

    fs::create_dir_all(data_dir).expect("Failed to create data directory.");

    info!("Beginning training...");

    training::train::<AutodiffBackend>(
        data_dir,
        CNNTrainingConfig::new(
            CNNModelConfig::new(dataset.full.num_labels()),
            AdamConfig::new(),
        ),
        dataset,
        device,
    );

    println!("Training complete");

    Ok(())
}

pub fn infer(
    data_dir: &str,
    recognise: &Interaction,
    interactions: Vec<Interaction>,
    relative_to: &MacAddress,
) -> Result<(), Error> {
    let device = WgpuDevice::default();
    let dataset = SplitNumericTraceDataset::split(
        NumericTraceDataset::load(interactions, relative_to)?,
        TRAINING_PROPORTION,
        VALIDATION_PROPORTION,
        TESTING_PROPORTION,
    )?;
    let recognised = inference::infer::<AutodiffBackend>(
        data_dir,
        NumericTraceDataset::load_trace(recognise, relative_to)?,
        device.clone(),
    );

    println!(
        "Recognised as: {}",
        dataset
            .full
            .get_query(recognised)
            .unwrap_or(&String::from("Unknown"))
    );

    Ok(())
}
