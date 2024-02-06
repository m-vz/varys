use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use log::{debug, info, warn};
use toml::Table;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub category: String,
}

impl Query {
    /// Read queries from a TOML file.
    ///
    /// The TOML file should have the following format:
    ///
    /// ```toml
    /// category_1 = ["query_1", "query_2"]
    /// category_2 = ["query_3"]
    /// ```
    ///
    /// # Arguments
    ///
    /// * `path`: The path to the TOML file.
    ///
    /// Returns a vec of the [`Query`]s found in the file.
    ///
    /// # Examples
    ///
    /// ```
    /// # use varys::query::Query;
    /// let queries = Query::read_toml("../data/test_queries.toml").unwrap();
    /// assert!(queries
    ///     .first()
    ///     .is_some_and(|query| query.category == "test_category_jokes"
    ///         && query.text == "Tell me a machine learning joke."));
    /// ```
    pub fn read_toml<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Error> {
        info!("Reading queries from {}", path.as_ref().display());

        let mut queries = Vec::new();
        let toml = fs::read_to_string(path)
            .map_err(|e| {
                warn!("Could not read queries file");

                Error::Io(e)
            })?
            .parse::<Table>()?;

        for (category, value) in toml {
            if let Some(array) = value.as_array() {
                for query in array {
                    if let Some(query) = query.as_str() {
                        queries.push(Query {
                            text: query.to_string(),
                            category: category.to_string(),
                        })
                    }
                }
            }
        }

        debug!("Found {} queries", queries.len());

        Ok(queries)
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.text, self.category)
    }
}
