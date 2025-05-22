use crate::storage::StorageEngine; // 引入存储引擎，crate表示根模块
use sqlparser::ast::{Query, Statement, SelectItem, Expr}; // 引入 SQL AST 类型
use crate::executor::error::ExecutionError; // 引入错误定义
use crate::storage::storage_engine_interface::Row;

/// 处理 SELECT 查询
pub fn handle_query(query: Box<Query>, storage_engine: &mut dyn StorageEngine) -> Result<(), ExecutionError> {
    // 提取目标表和列
    let table_name = extract_table_name(&query.body)?;
    let columns = extract_columns(&query.projection);
    let condition = extract_condition(&query.selection);

    // 调用存储引擎的 select_rows 方法
    let rows = storage_engine.select_rows(table_name, columns, condition)
        .map_err(|e| ExecutionError::StorageError(e.to_string()))?;

    // 输出查询结果
    if rows.is_empty() {
        println!("No results found.");
    } else {
        format_and_print_results(rows);
    }
    // OK(())表示返回一个空的Result对象
    Ok(())
}

/// 处理 INSERT 操作
pub fn handle_insert(statement: Statement, storage_engine: &mut dyn StorageEngine) -> Result<(), ExecutionError> {
    if let Statement::Insert { table_name, columns, source } = statement {
        // 提取插入的数据
        let rows = extract_insert_values(source)?;

        // 调用存储引擎的 insert_rows 方法
        storage_engine.insert_rows(&table_name.to_string(), &columns, rows)
            .map_err(|e| ExecutionError::StorageError(e.to_string()))?;

        println!("Insert successful!");
        Ok(())
    } else {
        Err(ExecutionError::UnsupportedStatement)
    }
}

/// 提取表名 (示例工具函数)
fn extract_table_name(body: &Query) -> Result<String, ExecutionError> {
    match body {
        Query::Select(select) => {
            if let Some(table) = select.from.iter().next() {
                Ok(table.to_string())
            } else {
                Err(ExecutionError::SyntaxError)
            }
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

/// 提取列名 (示例工具函数)
fn extract_columns(projection: &[SelectItem]) -> Vec<String> {
    projection.iter().map(|item| match item {
        SelectItem::UnnamedExpr(Expr::Identifier(ident)) => ident.to_string(),
        _ => "Unknown".to_string(),
    }).collect()
}

/// 提取 WHERE 条件 (示例工具函数)
fn extract_condition(_selection: &Option<Box<sqlparser::ast::Expr>>) -> Option<crate::storage::storage_engine_interface::Condition> {
    // 这里只做简单示例，实际应解析 SQL AST
    None
}

/// 提取插入的值 (示例工具函数)
fn extract_insert_values(_source: sqlparser::ast::Query) -> Result<Vec<crate::storage::storage_engine_interface::Row>, ExecutionError> {
    // 这里只做简单示例，实际应解析 SQL AST
    Ok(vec![crate::storage::storage_engine_interface::Row { values: vec![] }])
}

/// 格式化并输出结果
fn format_and_print_results(rows: Vec<Row>) {
    println!("Results:");
    for row in rows {
        println!("{:?}", row);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::storage_engine_interface::{Row, Value, Schema, ColumnDefinition, DataType, Constraint, Condition, StorageEngine};
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    use sqlparser::ast::{Query, Statement};

    struct MockStorageEngine;
    impl StorageEngine for MockStorageEngine {
        fn create_table(&mut self, _table_name: &str, _schema: Schema) -> Result<(), String> { Ok(()) }
        fn drop_table(&mut self, _table_name: &str) -> Result<(), String> { Ok(()) }
        fn insert_row(&mut self, _table_name: &str, _row: Row) -> Result<(), String> { Ok(()) }
        fn update_rows(&mut self, _table_name: &str, _updates: Vec<(String, Value)>, _condition: Option<Condition>) -> Result<u64, String> { Ok(1) }
        fn delete_rows(&mut self, _table_name: &str, _condition: Option<Condition>) -> Result<u64, String> { Ok(1) }
        fn select_rows(&self, _table_name: &str, _columns: Vec<String>, _condition: Option<Condition>) -> Result<Vec<Row>, String> {
            Ok(vec![Row { values: vec![Value::Int(1), Value::Varchar("Alice".to_string())] }])
        }
        fn get_table_schema(&self, _table_name: &str) -> Result<Schema, String> {
            Ok(Schema { columns: vec![
                ColumnDefinition { name: "id".to_string(), data_type: DataType::Int(32), constraints: vec![Constraint::PrimaryKey] },
                ColumnDefinition { name: "name".to_string(), data_type: DataType::Varchar(255), constraints: vec![] },
            ] })
        }
    }

    #[test]
    fn test_handle_query() {
        let sql = "SELECT id, name FROM users;";
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql).unwrap();
        if let Statement::Query(query) = &ast[0] {
            let mut storage = MockStorageEngine;
            let result = handle_query(query.clone(), &mut storage);
            assert!(result.is_ok());
        } else {
            panic!("Not a query statement");
        }
    }

    #[test]
    fn test_handle_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice');";
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql).unwrap();
        if let Statement::Insert { .. } = &ast[0] {
            let mut storage = MockStorageEngine;
            let result = handle_insert(ast[0].clone(), &mut storage);
            assert!(result.is_ok());
        } else {
            panic!("Not an insert statement");
        }
    }
}
