// lib.rs for executor crate
pub mod error;
pub mod formatter;
pub mod handler;
pub mod utils;

use crate::error::ExecutionError;
use sqlparser;
use storage::storage_engine_interface::StorageEngine;

pub fn execute_ast(
    ast: Vec<sqlparser::ast::Statement>,
    storage_engine: &mut dyn StorageEngine,
) -> Result<(), ExecutionError> {
    for statement in ast {
        // 调用 handler.rs 中的 execute_ast 来处理单个语句
        // '?'的意思是：如果前面的语句返回 Ok，则继续执行；如果返回 Err，则当前函数（execute_ast）也返回 Err。
        // - 如果是 Ok(QueryResult)，则 QueryResult 会被丢弃（因为表达式的结果未被赋值）。
        // - 如果是 Err(ExecutionError)，错误会向上传播，此函数将提前返回。
        handler::execute_stmt(statement, storage_engine)?;
    }
    Ok(())
}

// Re-export key components to be easily accessible by users of this crate
pub use handler::*;

#[cfg(test)]
pub mod tests {
    use super::handler::tests::MockExecutorStorageEngine;
    use super::handler::QueryResult;
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    use storage::storage_engine_interface::{DataType as StorageDataType, Value};

    fn parse_sql_to_statements(sql: &str) -> Vec<sqlparser::ast::Statement> {
        Parser::parse_sql(&GenericDialect {}, sql).unwrap()
    }

    #[test]
    fn test_execute_multiple_statements() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let sql = "CREATE TABLE multi_exec_test (id INT, name VARCHAR(100)); INSERT INTO multi_exec_test (id, name) VALUES (1, 'Test User');";
        let statements = parse_sql_to_statements(sql);

        let result = execute_ast(statements, &mut mock_storage);
        assert!(
            result.is_ok(),
            "Executing multiple statements failed: {:?}",
            result.err()
        );

        // Verify table creation
        let schema_result = mock_storage.get_table_schema("multi_exec_test");
        assert!(
            schema_result.is_ok(),
            "Table schema not found after CREATE TABLE"
        );
        let schema = schema_result.unwrap();
        assert_eq!(schema.columns.len(), 2);
        assert_eq!(schema.columns[0].name, "id");
        assert_eq!(schema.columns[0].data_type, StorageDataType::Int(32)); // Int default
        assert_eq!(schema.columns[1].name, "name");
        assert_eq!(schema.columns[1].data_type, StorageDataType::Varchar(100)); // Varchar(100)

        // Verify insertion
        let selected_rows_result = mock_storage.select_rows(
            "multi_exec_test",
            vec!["id".to_string(), "name".to_string()],
            None,
        );
        assert!(
            selected_rows_result.is_ok(),
            "Failed to select rows after INSERT"
        );
        let selected_rows = selected_rows_result.unwrap();
        assert_eq!(selected_rows.len(), 1, "Expected one row after INSERT");
        assert_eq!(selected_rows[0].values.len(), 2);
        assert_eq!(selected_rows[0].values[0], Value::Int(1));
        assert_eq!(
            selected_rows[0].values[1],
            Value::Varchar("Test User".to_string())
        );
    }

    #[test]
    fn test_execute_select_with_condition_in_mod() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        // Setup: Create table and insert data
        let setup_sql = "CREATE TABLE users (id INT, name VARCHAR(100), age INT); \
                         INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30); \
                         INSERT INTO users (id, name, age) VALUES (2, 'Bob', 24); \
                         INSERT INTO users (id, name, age) VALUES (3, 'Charlie', 30);";
        let setup_statements = parse_sql_to_statements(setup_sql);
        execute_ast(setup_statements, &mut mock_storage).expect("Setup failed");

        // Test: Select with a condition
        let select_sql = "SELECT id, name FROM users WHERE age = 30;";
        let select_statements = parse_sql_to_statements(select_sql);

        // We need to call handler::execute_ast for a single query to get QueryResult::Data
        // The execute_ast in lib.rs returns Result<(), ExecutionError>
        match handler::execute_stmt(select_statements[0].clone(), &mut mock_storage) {
            Ok(QueryResult::Data(rows)) => {
                assert_eq!(rows.len(), 2, "Expected two users with age 30");
                // Check first user (Alice)
                assert!(
                    rows.iter().any(|row| row.values[0] == Value::Int(1)
                        && row.values[1] == Value::Varchar("Alice".to_string())),
                    "Alice not found or incorrect data"
                );
                // Check second user (Charlie)
                assert!(
                    rows.iter().any(|row| row.values[0] == Value::Int(3)
                        && row.values[1] == Value::Varchar("Charlie".to_string())),
                    "Charlie not found or incorrect data"
                );
            }
            Err(e) => panic!("SELECT query execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for SELECT"),
        }
    }

    #[test]
    fn test_execute_error_handling() {
        let mut mock_storage = MockExecutorStorageEngine::new();

        // Test inserting into non-existent table
        let invalid_sql = "INSERT INTO non_existent_table (id) VALUES (1);";
        let statements = parse_sql_to_statements(invalid_sql);

        let result = execute_ast(statements, &mut mock_storage);
        assert!(
            result.is_err(),
            "Expected error when inserting into non-existent table"
        );
    }

    #[test]
    fn test_execute_empty_statements() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let empty_statements = Vec::new();

        let result = execute_ast(empty_statements, &mut mock_storage);
        assert!(result.is_ok(), "Empty statements should succeed");
    }

    #[test]
    fn test_execute_mixed_statements() {
        let mut mock_storage = MockExecutorStorageEngine::new();

        // Mix of CREATE, INSERT, and more operations
        let mixed_sql = "CREATE TABLE products (id INT, name VARCHAR(50), price INT); \
                        INSERT INTO products (id, name, price) VALUES (1, 'Laptop', 1200); \
                        INSERT INTO products (id, name, price) VALUES (2, 'Mouse', 25);";
        let statements = parse_sql_to_statements(mixed_sql);

        let result = execute_ast(statements, &mut mock_storage);
        assert!(
            result.is_ok(),
            "Mixed statements should succeed: {:?}",
            result.err()
        );

        // Verify the final state
        let select_result = mock_storage.select_rows(
            "products",
            vec!["id".to_string(), "name".to_string(), "price".to_string()],
            None,
        );
        assert!(select_result.is_ok());
        let rows = select_result.unwrap();
        assert_eq!(rows.len(), 2, "Expected two products");

        // Verify specific data
        assert!(
            rows.iter().any(|row| row.values[0] == Value::Int(1)
                && row.values[1] == Value::Varchar("Laptop".to_string())
                && row.values[2] == Value::Int(1200)),
            "Laptop not found with correct data"
        );

        assert!(
            rows.iter().any(|row| row.values[0] == Value::Int(2)
                && row.values[1] == Value::Varchar("Mouse".to_string())
                && row.values[2] == Value::Int(25)),
            "Mouse not found with correct data"
        );
    }

    #[test]
    fn test_execute_sql_with_comments() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试包含注释的完整SQL脚本
        let sql_script = "
            -- 创建用户表
            CREATE TABLE users_with_comments (
                id INT PRIMARY KEY, /* 用户ID，主键 */
                username VARCHAR(50) NOT NULL, -- 用户名，不能为空
                email VARCHAR(100) /* 邮箱地址 */
            );
            
            /* 插入测试用户数据 */
            INSERT INTO users_with_comments (id, username, email) VALUES 
                (1, 'admin', 'admin@example.com'), -- 管理员账户
                (2, 'user1', 'user1@example.com'); /* 普通用户 */
        ";
        
        let statements = parse_sql_to_statements(sql_script);
        let result = execute_ast(statements, &mut mock_storage);
        
        assert!(result.is_ok(), "带注释的SQL脚本应该执行成功: {:?}", result.err());
        
        // 验证结果
        let schema = mock_storage.get_table_schema("users_with_comments").unwrap();
        assert_eq!(schema.columns.len(), 3);
        
        let rows = mock_storage.select_rows(
            "users_with_comments",
            vec!["id".to_string(), "username".to_string(), "email".to_string()],
            None
        ).unwrap();
        assert_eq!(rows.len(), 2, "应该插入了两个用户");
    }

    #[test]
    fn test_execute_create_table_with_int_specifications() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试不同INT长度规格的混合SQL
        let sql_script = "
            CREATE TABLE data_types_test (
                tiny_id INT(1),        -- 1位整数
                small_id INT(16),      -- 16位整数  
                regular_id INT,        -- 默认长度整数
                big_id INT(128),       -- 128位整数
                description VARCHAR(200)  -- 描述字段
            );
            
            INSERT INTO data_types_test (tiny_id, small_id, regular_id, big_id, description) 
            VALUES (1, 1000, 50000, 9999999, 'Test record');
        ";
        
        let statements = parse_sql_to_statements(sql_script);
        let result = execute_ast(statements, &mut mock_storage);
        
        assert!(result.is_ok(), "INT长度规格测试应该执行成功: {:?}", result.err());
        
        // 验证表结构
        let schema = mock_storage.get_table_schema("data_types_test").unwrap();
        assert_eq!(schema.columns.len(), 5);
        
        // 验证各列的数据类型和长度
        assert_eq!(schema.columns[0].data_type, StorageDataType::Int(1));
        assert_eq!(schema.columns[1].data_type, StorageDataType::Int(16)); 
        assert_eq!(schema.columns[2].data_type, StorageDataType::Int(32)); // 默认
        assert_eq!(schema.columns[3].data_type, StorageDataType::Int(128));
        assert_eq!(schema.columns[4].data_type, StorageDataType::Varchar(200));
        
        // 验证数据插入
        let rows = mock_storage.select_rows(
            "data_types_test",
            vec!["tiny_id".to_string(), "small_id".to_string(), "regular_id".to_string(), "big_id".to_string()],
            None
        ).unwrap();
        assert_eq!(rows.len(), 1, "应该有一条测试记录");
    }
}
