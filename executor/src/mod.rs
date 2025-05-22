// Filename: mod.rs
// mod.rs是模块的入口文件
pub mod handler;
pub mod formatter;
pub mod error;
pub mod utils;

use crate::storage::StorageEngine;
use crate::error::ExecutionError;
use sqlparser;

pub fn execute_ast(ast: Vec<sqlparser::ast::Statement>, storage_engine: &mut dyn StorageEngine) -> Result<(), ExecutionError> {
    for statement in ast {
        match statement {
            sqlparser::ast::Statement::Query(query) => handler::handle_query(query, storage_engine)?,
            sqlparser::ast::Statement::Insert { .. } => handler::handle_insert(statement, storage_engine)?,
            sqlparser::ast::Statement::Update { .. } => handler::handle_update(statement, storage_engine)?,
            sqlparser::ast::Statement::Delete { .. } => handler::handle_delete(statement, storage_engine)?,
            _ => return Err(ExecutionError::UnsupportedStatement),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
