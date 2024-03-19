use burn::backend::wgpu::WgpuDevice;
use burn::config::Config;
use burn::data::dataloader::batcher::Batcher;
use burn::record::{CompactRecorder, Recorder};
use burn::tensor::backend::Backend;
use burn::tensor::Tensor;

use crate::error::Error;
use crate::ml::cnn::training::CNNTrainingConfig;
use crate::ml::data::{NumericTraceItem, TrafficTraceBatcher};
use crate::ml::{config_path, model_path, AutodiffBackend};
use crate::trace::NumericTrafficTrace;

pub fn predict(
    data_dir: &str,
    trace: NumericTrafficTrace,
    device: WgpuDevice,
) -> Result<u8, Error> {
    let output = infer::<AutodiffBackend>(data_dir, trace, device)?;
    let predicted: i32 = output.argmax(1).flatten::<1>(0, 1).into_scalar();

    Ok(predicted.try_into().unwrap())
}

pub fn infer<B: Backend<IntElem = i32>>(
    data_dir: &str,
    trace: NumericTrafficTrace,
    device: B::Device,
) -> Result<Tensor<B, 2>, Error> {
    let config = CNNTrainingConfig::load(config_path(data_dir))?;
    let record = CompactRecorder::new().load(model_path(data_dir).into(), &device)?;
    let model = config.model.init_with::<B>(record);
    let batcher = TrafficTraceBatcher::new(device);
    let batch = batcher.batch(vec![NumericTraceItem { trace, label: 0 }]);

    Ok(model.forward(batch.traces))
}
