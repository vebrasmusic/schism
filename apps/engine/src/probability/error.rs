use thiserror::Error;

/// errors that can arise when constructing or operating on a [`super::UnitInterval`]
#[derive(Debug, Error)]
#[error("probability error: {0}")]
pub struct ProbabilityError(pub String);
