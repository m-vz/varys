use burn::config::Config;
use burn::data::dataloader::batcher::Batcher;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::backend::Backend;

use crate::ml::cnn::training::CNNTrainingConfig;
use crate::ml::data::{NumericTraceItem, TrafficTraceBatcher};
use crate::trace::NumericTrafficTrace;

pub fn infer<B: Backend<IntElem = i32>>(
    data_dir: &str,
    trace: NumericTrafficTrace,
    device: B::Device,
) -> u8 {
    let config =
        CNNTrainingConfig::load(format!("{data_dir}/config.json")).expect("Failed to load config.");
    let record = CompactRecorder::new()
        .load(format!("{data_dir}/model").into(), &device)
        .expect("Failed to load model.");
    let model = config.model.init_with::<B>(record);
    let batcher = TrafficTraceBatcher::new(device);
    let batch = batcher.batch(vec![NumericTraceItem { trace, label: 0 }]);
    let output = model.forward(batch.traces);
    let predicted: i32 = output.argmax(1).flatten::<1>(0, 1).into_scalar();

    predicted.try_into().unwrap_or(0)
}
