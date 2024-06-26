use burn::config::Config;
use burn::module::Module;
use burn::nn::conv::{Conv1d, Conv1dConfig};
use burn::nn::pool::{AdaptiveAvgPool1d, AdaptiveAvgPool1dConfig};
use burn::nn::{Dropout, DropoutConfig, Linear, LinearConfig};
use burn::tensor::backend::Backend;
use burn::tensor::Tensor;

use crate::ml::activation::{Tanh, ELU, SELU};

pub mod inference;
pub mod training;

#[derive(Module, Debug)]
pub struct CNNModel<B: Backend> {
    convolution_0: Conv1d<B>,
    convolution_1: Conv1d<B>,
    convolution_2: Conv1d<B>,
    convolution_3: Conv1d<B>,
    pooling: AdaptiveAvgPool1d,
    dropout_0: Dropout,
    dropout_1: Dropout,
    dropout_2: Dropout,
    dense_0: Linear<B>,
    dense_1: Linear<B>,
    activation_tanh: Tanh,
    activation_elu: ELU,
    activation_selu: SELU,
}

impl<B: Backend> CNNModel<B> {
    pub fn forward(&self, traces: Tensor<B, 2>) -> Tensor<B, 2> {
        let [batch_size, trace_length] = traces.dims();

        // create a channel at the second dimension for compatibility with the convolution layers
        let x = traces.reshape([batch_size, 1, trace_length]);

        let x = self.convolution_0.forward(x);
        let x = self.activation_tanh.forward(x);
        let x = self.dropout_0.forward(x);

        // let x = self.convolution_1.forward(x);
        // let x = self.activation_elu.forward(x);
        // let x = self.dropout_1.forward(x);

        // let x = self.convolution_2.forward(x);
        // let x = self.activation_elu.forward(x);
        // let x = self.dropout_2.forward(x);
        //
        // let x = self.convolution_3.forward(x);
        // let x = self.activation_selu.forward(x);

        let x = self.pooling.forward(x);
        let [batch_size, channels, _] = x.dims();
        let x = x.reshape([batch_size, channels]);

        let x = self.dense_0.forward(x);
        let x = self.activation_elu.forward(x); // was SELU

        self.dense_1.forward(x)
        // we don't need to apply softmax here since the logits will be turned into probabilities by
        // the loss function
    }
}

#[derive(Config, Debug)]
pub struct CNNModelConfig {
    num_classes: usize,
    input_dimension: usize,
    #[config(default = 0.1)]
    dropout_rate_0: f64,
    #[config(default = 0.3)]
    dropout_rate_1: f64,
    #[config(default = 0.1)]
    dropout_rate_2: f64,
    #[config(default = 475)] // was 180
    dense_size: usize,
    #[config(default = 128)]
    convolution_number_0: usize,
    #[config(default = 128)]
    convolution_number_1: usize,
    #[config(default = 64)]
    convolution_number_2: usize,
    #[config(default = 256)]
    convolution_number_3: usize,
    #[config(default = 7)]
    filter_size_0: usize,
    #[config(default = 19)]
    filter_size_1: usize,
    #[config(default = 13)]
    filter_size_2: usize,
    #[config(default = 23)]
    filter_size_3: usize,
}

impl CNNModelConfig {
    pub const DEFAULT_INPUT_DIMENSIONS: usize = 475;

    pub fn init<B: Backend>(&self, device: &B::Device) -> CNNModel<B> {
        CNNModel {
            convolution_0: Conv1dConfig::new(1, self.convolution_number_0, self.filter_size_0)
                .init(device),
            convolution_1: Conv1dConfig::new(
                self.convolution_number_0,
                self.convolution_number_1,
                self.filter_size_1,
            )
            .init(device),
            convolution_2: Conv1dConfig::new(
                self.convolution_number_1,
                self.convolution_number_2,
                self.filter_size_2,
            )
            .init(device),
            convolution_3: Conv1dConfig::new(
                self.convolution_number_2,
                self.convolution_number_3,
                self.filter_size_3,
            )
            .init(device),
            pooling: AdaptiveAvgPool1dConfig::new(1).init(),
            dropout_0: DropoutConfig::new(self.dropout_rate_0).init(),
            dropout_1: DropoutConfig::new(self.dropout_rate_1).init(),
            dropout_2: DropoutConfig::new(self.dropout_rate_2).init(),
            dense_0: LinearConfig::new(self.convolution_number_0, self.dense_size).init(device), // was convolution_number_3
            dense_1: LinearConfig::new(self.dense_size, self.num_classes).init(device),
            activation_tanh: Tanh::new(),
            activation_elu: ELU::new(1.),
            activation_selu: SELU::new(),
        }
    }

    pub fn init_with<B: Backend>(self, record: CNNModelRecord<B>) -> CNNModel<B> {
        CNNModel {
            convolution_0: Conv1dConfig::new(1, self.convolution_number_0, self.filter_size_0)
                .init_with(record.convolution_0),
            convolution_1: Conv1dConfig::new(
                self.convolution_number_0,
                self.convolution_number_1,
                self.filter_size_1,
            )
            .init_with(record.convolution_1),
            convolution_2: Conv1dConfig::new(
                self.convolution_number_1,
                self.convolution_number_2,
                self.filter_size_2,
            )
            .init_with(record.convolution_2),
            convolution_3: Conv1dConfig::new(
                self.convolution_number_2,
                self.convolution_number_3,
                self.filter_size_3,
            )
            .init_with(record.convolution_3),
            pooling: AdaptiveAvgPool1dConfig::new(1).init(),
            dropout_0: DropoutConfig::new(self.dropout_rate_0).init(),
            dropout_1: DropoutConfig::new(self.dropout_rate_1).init(),
            dropout_2: DropoutConfig::new(self.dropout_rate_2).init(),
            dense_0: LinearConfig::new(self.convolution_number_0, self.dense_size)
                .init_with(record.dense_0), // was convolution_number_3
            dense_1: LinearConfig::new(self.dense_size, self.num_classes).init_with(record.dense_1),
            activation_tanh: Tanh::new(),
            activation_elu: ELU::new(1.),
            activation_selu: SELU::new(),
        }
    }
}
