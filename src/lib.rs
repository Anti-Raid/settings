pub mod cfg;
pub mod common_columns;
pub mod serenity;
pub mod types;
pub mod value;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
