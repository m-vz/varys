use burn::tensor::backend::Backend;
use burn::tensor::{Data, ElementConversion, Int, Tensor};

pub mod binary;
pub mod numeric;

pub struct TrafficTraceBatcher<B: Backend> {
    device: B::Device,
}

impl<B: Backend> TrafficTraceBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}

fn build_targets<B: Backend>(labels: Vec<u8>, device: &B::Device) -> Tensor<B, 1, Int> {
    let targets = labels
        .iter()
        // in this step we convert each item to the backend element type
        .map(|&label| Data::from([(label as i64).elem()]))
        .map(|data| Tensor::<B, 1, Int>::from_data(data, device))
        .collect();

    Tensor::cat(targets, 0).to_device(device)
}
