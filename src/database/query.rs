use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use log::warn;
use toml::Table;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub category: String,
}

impl Query {
    pub fn read_toml<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Error> {
        let mut queries = Vec::new();
        let toml = fs::read_to_string(path)
            .map_err(|e| {
                warn!("Could not read queries.");

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

        Ok(queries)
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.text, self.category)
    }
}
