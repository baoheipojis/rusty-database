#[derive(Debug)]
pub enum ExecutionError {
    StorageError(String),
    SyntaxError,
    UnsupportedStatement,
    TableNotFound(String),
    ColumnNotFound(String),
    // Add other error variants as needed
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::StorageError(s) => write!(f, "Storage error: {}", s),
            ExecutionError::SyntaxError => write!(f, "Syntax error"),
            ExecutionError::UnsupportedStatement => write!(f, "Unsupported statement"),
            ExecutionError::TableNotFound(table) => write!(f, "Table not found: {}", table),
            ExecutionError::ColumnNotFound(col) => write!(f, "Column not found: {}", col),
        }
    }
}

impl std::error::Error for ExecutionError {}
