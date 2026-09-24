use alloc::string::String;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ThreadError {
    Other(String),
}
