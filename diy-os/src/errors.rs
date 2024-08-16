use thiserror::Error;

pub mod validity {
    use super::Error;

    #[derive(Error, Debug)]
    #[error("{value} was out of range from {min} to {max}")]
    pub struct InputOutOfRangeInclusive<T> {
        pub max: T,
        pub min: T,
        pub value: T,
    }
}
