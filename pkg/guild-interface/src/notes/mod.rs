/// Interface for POST /notes
pub mod create;
/// Interface for GET /notes
pub mod list;
/// Core note interface datatypes
pub mod note;
/// Interface for POST /notes/request
pub mod request;

#[cfg(test)]
mod tests;
