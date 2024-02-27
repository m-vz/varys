#![allow(dead_code)]

use burn::module::Module;
use burn::tensor::activation::{softmax, tanh};
use burn::tensor::backend::Backend;
use burn::tensor::Tensor;

/// Applies the scaled exponential linear unit function element-wise:
///
/// `f(x) = λx if x > 0`
///
/// `f(x) = λα(exp(x) - 1) if x <= 0`
///
/// where `λ = 1.0507009873554804934193349852946` and `α = 1.6732632423543772848170429916717`.
///
/// The constants used in the implementation are 64-bit floats and thus not as precise as the above
/// constants.
///
/// The implementation was taken from [here](https://github.com/pytorch/pytorch/blob/96aaa311c0251d24decb9dc5da4957b7c590af6f/torch/nn/modules/activation.py#L507).
#[allow(clippy::upper_case_acronyms)]
#[derive(Module, Clone, Debug, Default)]
pub struct SELU {}

impl SELU {
    const LAMBDA: f64 = 1.050_700_987_355_480_5;
    const ALPHA: f64 = 1.673_263_242_354_377_2;

    /// Create the module.
    pub fn new() -> Self {
        Self {}
    }

    /// Applies the forward pass on the input tensor.
    ///
    /// # Shapes
    ///
    /// - input: `[..., any]`
    /// - output: `[..., any]`
    pub fn forward<B: Backend, const D: usize>(&self, input: Tensor<B, D>) -> Tensor<B, D> {
        let exponential_part = (input.clone().exp() - 1.0) * Self::ALPHA;
        let mask = input.clone().lower_equal_elem(0);

        input.mask_where(mask, exponential_part) * Self::LAMBDA
    }
}

/// Applies the scaled exponential linear unit function element-wise:
///
/// `f(x) = x if x > 0`
///
/// `f(x) = α(exp(x) - 1) else`
#[allow(clippy::upper_case_acronyms)]
#[derive(Module, Clone, Debug, Default)]
pub struct ELU {
    alpha: f64,
}

impl ELU {
    /// Create the module.
    pub fn new(alpha: f64) -> Self {
        Self { alpha }
    }

    /// Applies the forward pass on the input tensor.
    ///
    /// # Shapes
    ///
    /// - input: `[..., any]`
    /// - output: `[..., any]`
    pub fn forward<B: Backend, const D: usize>(&self, input: Tensor<B, D>) -> Tensor<B, D> {
        let exponential_part = (input.clone().exp() - 1.0) * self.alpha;
        let mask = input.clone().lower_equal_elem(0);

        input.mask_where(mask, exponential_part)
    }
}

/// Applies the softmax function element-wise.
#[derive(Module, Clone, Debug, Default)]
pub struct Softmax {
    dim: usize,
}

impl Softmax {
    /// Create the module.
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    /// Applies the forward pass on the input tensor.
    ///
    /// # Shapes
    ///
    /// - input: `[..., any]`
    /// - output: `[..., any]`
    pub fn forward<B: Backend, const D: usize>(&self, input: Tensor<B, D>) -> Tensor<B, D> {
        softmax(input, self.dim)
    }
}

/// Applies the softmax function element-wise.
#[derive(Module, Clone, Debug, Default)]
pub struct Tanh {}

impl Tanh {
    /// Create the module.
    pub fn new() -> Self {
        Self {}
    }

    /// Applies the forward pass on the input tensor.
    ///
    /// # Shapes
    ///
    /// - input: `[..., any]`
    /// - output: `[..., any]`
    pub fn forward<B: Backend, const D: usize>(&self, input: Tensor<B, D>) -> Tensor<B, D> {
        tanh(input)
    }
}
