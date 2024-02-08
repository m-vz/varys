use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot turn an empty list of packets into a trace")]
    EmptyTrace,
    #[error("At most {0} labels are supported")]
    TooManyLabels(usize),
}
