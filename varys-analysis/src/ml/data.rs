use burn::tensor::backend::Backend;

pub mod binary;

pub struct TrafficTraceBatcher<B: Backend> {
    device: B::Device,
}

impl<B: Backend> TrafficTraceBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}
