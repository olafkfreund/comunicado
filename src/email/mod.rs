pub mod message;
pub mod thread;
pub mod threading_engine;
pub mod sorting;

pub use message::{EmailMessage, MessageId};
pub use thread::{EmailThread, ThreadStatistics};
pub use threading_engine::{ThreadingEngine, ThreadingAlgorithm};
pub use sorting::{SortCriteria, SortOrder, MultiCriteriaSorter};