use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use burn::data::dataloader::batcher::Batcher;
use burn::data::dataset::transform::PartialDataset;
use burn::data::dataset::Dataset;
use burn::tensor::backend::Backend;
use burn::tensor::{Data, ElementConversion, Int, Tensor};
use log::{debug, info};
use serde::{Deserialize, Serialize};

use varys_database::database::interaction::Interaction;
use varys_database::file;
use varys_network::address::MacAddress;
use varys_network::packet;

use crate::error::Error;
use crate::ml;
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

impl NumericTraceItem {
    /// Resize the item, truncating if it is longer than `len` and adding zeroes if it is shorter.
    ///
    /// # Arguments
    ///
    /// * `len`: The new length of the item.
    ///
    /// # Examples
    ///
    /// See [`NumericTrafficTrace::resize`].
    pub fn resize(&mut self, len: usize) {
        self.trace.resize(len);
    }
}

#[derive(Deserialize, Serialize)]
pub struct NumericTraceDataset {
    pub items: Vec<NumericTraceItem>,
    /// The label of a query is the index of the query in this vector
    pub queries: Vec<String>,
}

impl NumericTraceDataset {
    const MAX_LABELS: usize = u8::MAX as usize;

    /// Load a dataset from disk, if it is found or create it from a list of [`Interaction`]s.
    ///
    /// If no existing dataset is found, a new one is created and saved to disk.
    ///
    /// # Arguments
    ///
    /// * `data_path`: The path to the data directory.
    /// * `interactions`: The interactions to create the dataset from if no dataset is found on disk.
    pub fn load_or_new<P: AsRef<Path>>(
        data_path: P,
        interactions: Vec<Interaction>,
    ) -> Result<NumericTraceDataset, Error> {
        let dataset_path = ml::dataset_path(&data_path);
        if dataset_path.exists() {
            NumericTraceDataset::load(&dataset_path)
        } else {
            let dataset = NumericTraceDataset::new(data_path, interactions)?;
            dataset.save(&dataset_path)?;
            Ok(dataset)
        }
    }

    /// Create a dataset of all numeric traffic traces from a list of interactions.
    ///
    /// Filters the interactions according to [`Self::filter_interactions`] and drops any interactions where the trace
    /// could not be loaded.
    ///
    /// # Arguments
    ///
    /// * `data_path`: The path to the data directory.
    /// * `interactions`: The interactions to create the dataset from.
    ///
    /// returns: The created dataset or [`Error::TooManyLabels`] if there were too many different queries.
    pub fn new<P: AsRef<Path>>(
        data_path: P,
        interactions: Vec<Interaction>,
    ) -> Result<Self, Error> {
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
                    Self::load_trace(&data_path, &interaction),
                    dataset.get_label(&interaction.query),
                )
            })
            // only keep items where the trace could be loaded and the label was found
            .filter_map(|(trace, label)| trace.ok().zip(label))
            .map(|(trace, label)| NumericTraceItem { trace, label })
            .collect();

        Ok(dataset)
    }

    /// Load a numeric traffic trace dataset from a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path`: The path to the JSON file.
    ///
    /// returns: The loaded dataset or an error if the file could not be opened or the JSON could not be deserialized.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        debug!("Loading dataset from {}", path.as_ref().display());

        Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
    }

    /// Save the dataset to a JSON file.
    ///
    /// # Arguments
    ///
    /// * `path`: Where to save the dataset.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?
            .write_all(serde_json::to_string(self)?.as_bytes())?;

        Ok(())
    }

    /// Resize all items in this dataset, truncating if they are longer than `len` and adding zeroes
    /// if they are shorter.
    ///
    /// # Arguments
    ///
    /// * `len`: The new length of all items in the dataset.
    ///
    /// # Examples
    ///
    /// See [`NumericTrafficTrace::resize`].
    pub fn resize_all(&mut self, len: usize) {
        self.items.iter_mut().for_each(|item| item.resize(len));
    }

    /// Find the query corresponding to a label. The label corresponds to the index of the query in the list of queries.
    ///
    /// # Arguments
    ///
    /// * `label`: The label to find the query for.
    ///
    /// returns: The query corresponding to the label or `None` if the label could not be found.
    pub fn get_query(&self, label: u8) -> Option<String> {
        self.queries.get(label as usize).cloned()
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

    /// Get the number of labels in the dataset.
    pub fn num_labels(&self) -> usize {
        self.queries.len()
    }

    /// Load a [`TrafficTrace`] from a pcap file.
    ///
    /// # Arguments
    ///
    /// * `data_path`: The path to the data directory.
    /// * `interaction`: The interaction to load the traffic trace from.
    ///
    /// returns: The parsed [`TrafficTrace`] or `None` if the pcap file could not be loaded.
    pub fn load_trace<P: AsRef<Path>>(
        data_path: P,
        interaction: &Interaction,
    ) -> Result<NumericTrafficTrace, Error> {
        let address =
            MacAddress::from_str(&interaction.assistant_mac).map_err(|_| Error::CannotLoadTrace)?;

        interaction
            .capture_file
            .clone()
            .map(|path| file::session_path(data_path, interaction.session_id).join(path))
            .and_then(|path| packet::load_packets(path).ok())
            .map(TrafficTrace::try_from)
            .transpose()?
            .map(|trace| trace.as_numeric_trace(&address))
            .ok_or(Error::CannotLoadTrace)
    }

    /// This function filters out all interactions that should not be used in the dataset.
    ///
    /// # Arguments
    ///
    /// * `interactions`: The interactions to filter.
    fn filter_interactions(interactions: Vec<Interaction>) -> Vec<Interaction> {
        interactions
            .into_iter()
            .filter(|interaction| interaction.is_complete())
            .collect()
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
    pub full: Arc<NumericTraceDataset>,
    pub training: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
    pub validation: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
    pub testing: PartialDataset<Arc<NumericTraceDataset>, NumericTraceItem>,
}

impl SplitNumericTraceDataset {
    const TRAINING_PROPORTION: f64 = 0.64;
    const VALIDATION_PROPORTION: f64 = 0.16;
    const TESTING_PROPORTION: f64 = 0.2;

    /// Split a [`NumericTraceDataset`] into training, validation, and testing datasets using the
    /// default proportions.
    ///
    /// # Arguments
    ///
    /// * `dataset`: The dataset to split.
    pub fn split_default(dataset: NumericTraceDataset) -> Result<Self, Error> {
        Self::split(
            dataset,
            Self::TRAINING_PROPORTION,
            Self::VALIDATION_PROPORTION,
            Self::TESTING_PROPORTION,
        )
    }

    /// Split a [`NumericTraceDataset`] into training, validation, and testing datasets.
    ///
    /// # Arguments
    ///
    /// * `dataset`: The dataset to split.
    /// * `training_proportion`: The proportion of the dataset to use for training.
    /// * `validation_proportion`: The proportion of the dataset to use for validation.
    /// * `testing_proportion`: The proportion of the dataset to use for testing.
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

        if validation_index <= 1
            || testing_index <= validation_index + 1
            || dataset.len() <= testing_index + 1
        {
            return Err(Error::DatasetTooSmall);
        }

        Ok(SplitNumericTraceDataset {
            full: dataset.clone(),
            training: PartialDataset::new(dataset.clone(), 0, validation_index - 1),
            validation: PartialDataset::new(dataset.clone(), validation_index, testing_index - 1),
            testing: PartialDataset::new(dataset.clone(), testing_index, dataset.len() - 1),
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
            .map(|item| Data::<f32, 1>::from(item.trace.0.as_slice()))
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
            .map(|item| Data::from([(item.label as i64).elem()]))
            .map(|data| Tensor::<B, 1, Int>::from_data(data, &self.device))
            .collect();
        let targets = Tensor::cat(targets, 0).to_device(&self.device);

        NumericBatch { traces, targets }
    }
}
