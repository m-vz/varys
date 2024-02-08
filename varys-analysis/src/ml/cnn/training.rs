use std::fs;

use burn::config::Config;
use burn::data::dataloader::DataLoaderBuilder;
use burn::lr_scheduler::noam::NoamLrSchedulerConfig;
use burn::module::Module;
use burn::nn::loss::CrossEntropyLossConfig;
use burn::optim::AdamConfig;
use burn::record::CompactRecorder;
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::{Int, Tensor};
use burn::train::metric::{AccuracyMetric, LossMetric};
use burn::train::{ClassificationOutput, LearnerBuilder, TrainOutput, TrainStep, ValidStep};

use crate::ml::cnn::{CNNModel, CNNModelConfig};
use crate::ml::data::{NumericBatch, SplitNumericTraceDataset, TrafficTraceBatcher};

impl<B: AutodiffBackend> TrainStep<NumericBatch<B>, ClassificationOutput<B>> for CNNModel<B> {
    fn step(&self, batch: NumericBatch<B>) -> TrainOutput<ClassificationOutput<B>> {
        let item = self.forward_classification(batch.traces, batch.targets);

        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<NumericBatch<B>, ClassificationOutput<B>> for CNNModel<B> {
    fn step(&self, batch: NumericBatch<B>) -> ClassificationOutput<B> {
        self.forward_classification(batch.traces, batch.targets)
    }
}

impl<B: Backend> CNNModel<B> {
    pub fn forward_classification(
        &self,
        traces: Tensor<B, 2>,
        targets: Tensor<B, 1, Int>,
    ) -> ClassificationOutput<B> {
        let output = self.forward(traces);
        let loss = CrossEntropyLossConfig::new()
            .init(&output.device())
            .forward(output.clone(), targets.clone());

        ClassificationOutput::new(loss, output, targets)
    }
}

#[derive(Config)]
pub struct CNNTrainingConfig {
    pub model: CNNModelConfig,
    pub optimizer: AdamConfig,
    #[config(default = 10)]
    pub num_epochs: usize,
    #[config(default = 70)]
    pub batch_size: usize,
    #[config(default = 8)]
    pub num_workers: usize,
    #[config(default = 42)]
    pub seed: u64,
    #[config(default = 0.002)]
    pub learning_rate: f64,
    #[config(default = 0.13)]
    pub decay: f64,
}

pub fn train<B: AutodiffBackend>(
    data_dir: &str,
    config: CNNTrainingConfig,
    dataset: SplitNumericTraceDataset,
    device: B::Device,
) {
    fs::create_dir_all(data_dir).expect("Failed to create artifact directory.");
    config
        .save(format!("{data_dir}/config.json"))
        .expect("Failed to save config.");

    B::seed(config.seed);

    let batcher_train = TrafficTraceBatcher::<B>::new(device.clone());
    let batcher_valid = TrafficTraceBatcher::<B::InnerBackend>::new(device.clone());
    let data_loader_training = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(dataset.training);
    let data_loader_validation = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(dataset.validation);
    let learner = LearnerBuilder::new(data_dir)
        .metric_train_numeric(AccuracyMetric::new())
        .metric_valid_numeric(AccuracyMetric::new())
        .metric_train_numeric(LossMetric::new())
        .metric_valid_numeric(LossMetric::new())
        .with_file_checkpointer(CompactRecorder::new())
        .devices(vec![device.clone()])
        .num_epochs(config.num_epochs)
        .build(
            config.model.init::<B>(&device),
            config.optimizer.init(),
            NoamLrSchedulerConfig::new(config.learning_rate).init(),
        );

    learner
        .fit(data_loader_training, data_loader_validation)
        .save_file(format!("{data_dir}/model"), &CompactRecorder::new())
        .expect("Failed to save trained model.");
}
