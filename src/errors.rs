use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Could not read the cache directory: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache for this manga does not exist or is not valid.")]
    CacheNotFound,

    #[error("The cache file is expired.")]
    CacheExpired,

    #[error("Failed to parse JSON content: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Error, Debug)]
pub enum SerieParseError {
    #[error("Document do not contain title element.")]
    MissingTitle,
    #[error("Fragment parsed do not contain scan name element.")]
    MissingScan,
}
