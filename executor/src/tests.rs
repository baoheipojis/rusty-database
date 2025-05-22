// executor/src/tests.rs
// 单元测试 for executor 模块

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    use sqlparser::ast::Statement;
    use crate::storage::storage_engine_interface::{Schema, ColumnDefinition, DataType, Constraint, Row, Value, Condition, StorageEngine};

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
    fn test_execute_ast_with_query() {
        let sql = "SELECT * FROM users;";
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql).unwrap();
        let mut storage = MockStorageEngine;
        let result = execute_ast(ast, &mut storage);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_ast_with_insert() {
        let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice');";
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql).unwrap();
        let mut storage = MockStorageEngine;
        let result = execute_ast(ast, &mut storage);
        assert!(result.is_ok());
    }
}
