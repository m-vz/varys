use std::sync::Arc;

use burn::data::dataloader::batcher::Batcher;
use burn::data::dataset::transform::PartialDataset;
use burn::data::dataset::Dataset;
use burn::tensor::backend::Backend;
use burn::tensor::{Data, ElementConversion, Int, Tensor};
use log::{debug, info};
use serde::{Deserialize, Serialize};

use varys_database::database::interaction::Interaction;
use varys_network::address::MacAddress;
use varys_network::packet::load_packets;

use crate::error::Error;
use crate::trace::{NumericTrafficTrace, TrafficTrace};

pub struct TrafficTraceBatcher<B: Backend> {
    device: B::Device,
}

impl<B: Backend> TrafficTraceBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { device }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NumericTraceItem {
    pub trace: NumericTrafficTrace,
    pub label: u8,
}

pub struct NumericTraceDataset {
    pub items: Vec<NumericTraceItem>,
    /// The label of a query is the index of the query in this vector
    queries: Vec<String>,
}

impl NumericTraceDataset {
    const MAX_LABELS: usize = u8::MAX as usize;

    /// Create a dataset of all numeric traffic traces from a list of interactions.
    ///
    /// Filters the interactions according to [`Self::filter_interactions`] and drops any interactions where the trace
    /// could not be loaded.
    ///
    /// # Arguments
    ///
    /// * `interactions`: The interactions to create the dataset from.
    /// * `relative_to`: The MAC address to use as the reference point for the trace.
    ///
    /// returns: The created dataset or [`Error::TooManyLabels`] if there were too many different queries.
    pub fn load(interactions: Vec<Interaction>, relative_to: &MacAddress) -> Result<Self, Error> {
        info!(
            "Creating dataset from {} interactions...",
            interactions.len()
        );

        let interactions = Self::filter_interactions(interactions);
        let mut dataset = Self {
            items: Vec::new(),
            queries: Self::collect_queries(&interactions)?,
        };

        dataset.items = interactions
            .into_iter()
            .map(|interaction| {
                (
                    Self::load_trace(&interaction, relative_to),
                    dataset.get_label(&interaction.query),
                )
            })
            // only keep items where the trace could be loaded and the label was found
            .filter_map(|(trace, label)| trace.zip(label))
            .map(|(trace, label)| NumericTraceItem { trace, label })
            .collect();

        Ok(dataset)
    }

    /// Find the query corresponding to a label. The label corresponds to the index of the query in the list of queries.
    ///
    /// # Arguments
    ///
    /// * `label`: The label to find the query for.
    ///
    /// returns: The query corresponding to the label or `None` if the label could not be found.
    pub fn get_query(&self, label: u8) -> Option<&String> {
        self.queries.get(label as usize)
    }

    /// Find the label of a query. This will search the list of queries.
    ///
    /// # Arguments
    ///
    /// * `query`: The query to find the label of.
    ///
    /// returns: The label of the query or `None` if the query could not be found.
    pub fn get_label(&self, query: &str) -> Option<u8> {
        self.queries
            .iter()
            .position(|label| label == query)
            .map(|label| label as u8)
    }

    /// This function filters out all interactions that should not be used in the dataset.
    ///
    /// # Arguments
    ///
    /// * `interactions`: The interactions to filter.
    fn filter_interactions(interactions: Vec<Interaction>) -> Vec<Interaction> {
        interactions
            .into_iter()
            .filter(|interaction| true)
            .collect()
    }

    /// Load a [`TrafficTrace`] from a pcap file.
    ///
    /// # Arguments
    ///
    /// * `interaction`: The interaction to load the traffic trace from.
    ///
    /// returns: The parsed [`TrafficTrace`] or `None` if the pcap file could not be loaded.
    fn load_trace(
        interaction: &Interaction,
        relative_to: &MacAddress,
    ) -> Option<NumericTrafficTrace> {
        interaction
            .capture_file
            .map(|file| load_packets(file).ok())
            .flatten()
            .map(|packets| TrafficTrace::try_from(packets).ok())
            .flatten()
            .map(|trace| trace.as_numeric_trace(relative_to))
    }

    /// Turns a list of interactions into a list of unique queries. The indices of the returned list will be used as
    /// labels.
    ///
    /// Returns an error if there are more than [`Self::MAX_LABELS`] unique queries.
    ///
    /// # Arguments
    ///
    /// * `interactions`: The list of interactions to search for queries.
    fn collect_queries(interactions: &Vec<Interaction>) -> Result<Vec<String>, Error> {
        let mut labels = Vec::with_capacity(Self::MAX_LABELS);

        for interaction in interactions {
            if !labels.contains(&interaction.query) {
                labels.push(interaction.query.clone());
            }

            if labels.len() > Self::MAX_LABELS {
                return Err(Error::TooManyLabels(Self::MAX_LABELS));
            }
        }

        debug!("Found {} unique queries", labels.len());

        Ok(labels)
    }
}

impl Dataset<NumericTraceItem> for NumericTraceDataset {
    fn get(&self, index: usize) -> Option<NumericTraceItem> {
        self.items.get(index).cloned()
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub struct SplitNumericTraceDataset {
    pub training: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
    pub validation: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
    pub testing: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
}

impl SplitNumericTraceDataset {
    pub fn split(
        dataset: NumericTraceDataset,
        training_proportion: f64,
        validation_proportion: f64,
        testing_proportion: f64,
    ) -> Result<Self, Error> {
        if !(0.0..1.0).contains(&training_proportion)
            || !(0.0..1.0).contains(&validation_proportion)
            || !(0.0..1.0).contains(&testing_proportion)
        {
            return Err(Error::ProportionError);
        }
        if (training_proportion + validation_proportion + testing_proportion - 1.).abs() > 0.001 {
            return Err(Error::ProportionSumError);
        }

        info!(
            "Splitting dataset into training: {:.0}%, validation: {:.0}%, testing: {:.0}%",
            (training_proportion * 100.).round(),
            (validation_proportion * 100.).round(),
            (testing_proportion * 100.).round()
        );

        let dataset = Arc::new(dataset);
        let length = dataset.len() as f64;
        let validation_index = (training_proportion * length) as usize;
        let testing_index = validation_index + (validation_proportion * length) as usize;

        Ok(SplitNumericTraceDataset {
            training: PartialDataset::new(dataset.clone(), 0, (validation_index - 1).max(0)),
            validation: PartialDataset::new(
                dataset.clone(),
                validation_index,
                (testing_index - 1).max(0),
            ),
            testing: PartialDataset::new(dataset, testing_index, dataset.len() - 1),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NumericBatch<B: Backend> {
    pub traces: Tensor<B, 2>,
    pub targets: Tensor<B, 1, Int>,
}

impl<B: Backend> Batcher<NumericTraceItem, NumericBatch<B>> for TrafficTraceBatcher<B> {
    fn batch(&self, items: Vec<NumericTraceItem>) -> NumericBatch<B> {
        let traces = items
            .iter()
            .map(|item| Data::<i32, 1>::from(item.trace.0.as_slice()))
            // in this step we convert all data to the backend type
            .map(|data| Tensor::<B, 1>::from_data(data.convert(), &self.device))
            .map(|tensor| {
                let size = tensor.shape().dims[0];
                tensor.reshape([1, size])
            })
            .collect();
        let traces = Tensor::cat(traces, 0).to_device(&self.device);
        let targets = items
            .iter()
            // in this step we convert each item to the backend element type
            .map(|&item| Data::from([(item.label as i64).elem()]))
            .map(|data| Tensor::<B, 1, Int>::from_data(data, &self.device))
            .collect();
        let targets = Tensor::cat(targets, 0).to_device(&self.device);

        NumericBatch { traces, targets }
    }
}
