use burn::config::Config;
use burn::data::dataloader::DataLoaderBuilder;
use burn::module::Module;
use burn::nn::loss::CrossEntropyLossConfig;
use burn::optim::AdamConfig;
use burn::record::CompactRecorder;
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::{Int, Tensor};
use burn::train::metric::{AccuracyMetric, LossMetric};
use burn::train::{ClassificationOutput, LearnerBuilder, TrainOutput, TrainStep, ValidStep};

use crate::error::Error;
use crate::ml::cnn::{CNNModel, CNNModelConfig};
use crate::ml::data::{NumericBatch, NumericTraceDataset, TrafficTraceBatcher};
use crate::ml::{config_path, ml_path, model_path};

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
    #[config(default = 1000)]
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
    training_dataset: NumericTraceDataset,
    validation_dataset: NumericTraceDataset,
    device: B::Device,
) -> Result<(), Error> {
    config.save(config_path(data_dir))?;

    B::seed(config.seed);

    let batcher_train = TrafficTraceBatcher::<B>::new(device.clone());
    let batcher_valid = TrafficTraceBatcher::<B::InnerBackend>::new(device.clone());
    let data_loader_training = DataLoaderBuilder::new(batcher_train)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(training_dataset);
    let data_loader_validation = DataLoaderBuilder::new(batcher_valid)
        .batch_size(config.batch_size)
        .shuffle(config.seed)
        .num_workers(config.num_workers)
        .build(validation_dataset);
    let learner = LearnerBuilder::new(&ml_path(data_dir))
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
            config.learning_rate,
        );

    learner
        .fit(data_loader_training, data_loader_validation)
        .save_file(model_path(data_dir), &CompactRecorder::new())
        .map_err(Error::from)
}
