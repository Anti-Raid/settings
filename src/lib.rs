pub mod cfg;
pub mod common_columns;
pub mod serenity;
pub mod types;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
