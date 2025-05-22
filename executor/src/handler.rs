use storage::storage_engine_interface::{StorageEngine, Row, Value, Condition, Schema, ColumnDefinition, DataType as StorageDataType}; // Renamed DataType to avoid conflict
use sqlparser::ast::{Statement, Query, SetExpr, SelectItem, Expr, TableFactor, Ident, ObjectName, BinaryOperator, DataType as SQLDataType}; // Renamed DataType, removed unused Values
use crate::error::ExecutionError;

// Represents the result of a query execution
#[derive(Debug)]
pub enum QueryResult {
    Data(Vec<Row>),
    RowsAffected(u64),
    Success, // For statements like CREATE TABLE, INSERT without returning data
}

pub fn execute_ast(statement: Statement, storage_engine: &mut dyn StorageEngine) -> Result<QueryResult, ExecutionError> {
    match statement {
        Statement::Query(query) => {
            let rows = handle_query(&query, storage_engine)?; // Pass Box<Query> by reference
            Ok(QueryResult::Data(rows))
        }
        Statement::Insert { table_name, columns, source, .. } => {
            handle_insert(&table_name, &columns, &source, storage_engine)?; // Pass Box<Query> by reference
            Ok(QueryResult::Success)
        }
        Statement::CreateTable { name, columns: ast_columns, .. } => {
            let table_name_str = name.0.get(0).ok_or(ExecutionError::SyntaxError)?.value.clone();
            let mut schema_columns = Vec::new();
            for col_def in ast_columns {
                let column_name = col_def.name.value.clone();
                let data_type = match col_def.data_type {
                    SQLDataType::Int(_) => StorageDataType::Int(32),
                    SQLDataType::Varchar(_) => StorageDataType::Varchar(255),
                    SQLDataType::SmallInt(_) => StorageDataType::Int(16),
                    SQLDataType::BigInt(_) => StorageDataType::Int(64),
                    SQLDataType::Char(_) => StorageDataType::Varchar(255), // Map Char to Varchar
                    SQLDataType::Decimal(_, _) => StorageDataType::Varchar(255), // Map Decimal to Varchar for simplicity
                    SQLDataType::Float(_) => StorageDataType::Varchar(255), // Map Float to Varchar for simplicity
                    SQLDataType::Real => StorageDataType::Varchar(255), // Map Real to Varchar for simplicity
                    SQLDataType::Double => StorageDataType::Varchar(255), // Map Double to Varchar for simplicity
                    SQLDataType::Boolean => StorageDataType::Varchar(5), // Map Boolean to Varchar("true"/"false")
                    SQLDataType::Date => StorageDataType::Varchar(10), // Map Date to Varchar (YYYY-MM-DD)
                    SQLDataType::Time => StorageDataType::Varchar(8), // Corrected: Time variant without arguments
                    SQLDataType::Timestamp => StorageDataType::Varchar(29), // Corrected: Timestamp variant without arguments
                    // Add other specific sqlparser DataType variants as they are used or needed
                    _ => return Err(ExecutionError::UnsupportedStatement), // Catch-all for unhandled types
                };
                schema_columns.push(ColumnDefinition {
                    name: column_name,
                    data_type,
                    constraints: Vec::new(), // TODO: Parse constraints
                });
            }
            let schema = Schema { columns: schema_columns };
            storage_engine.create_table(&table_name_str, schema)
                .map_err(ExecutionError::StorageError)?;
            Ok(QueryResult::Success)
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn handle_query(query: &Query, storage_engine: &dyn StorageEngine) -> Result<Vec<Row>, ExecutionError> {
    // Dereference Box<SetExpr> to get &SetExpr for helper functions
    let query_body_ref: &SetExpr = &query.body;
    let table_name = extract_table_name(query_body_ref)?;
    let columns = extract_columns_from_select(query_body_ref)?;
    let condition = extract_condition_from_select(query_body_ref)?;

    storage_engine.select_rows(&table_name, columns, condition)
        .map_err(ExecutionError::StorageError)
}

// Removed unused columns_idents parameter
fn handle_insert(table_name_obj: &ObjectName, _columns_idents: &[Ident], source_query: &Query, storage_engine: &mut dyn StorageEngine) -> Result<(), ExecutionError> {
    let table_name_str = table_name_obj.0.get(0).ok_or(ExecutionError::SyntaxError)?.value.clone();

    // Dereference Box<SetExpr> to get &SetExpr for helper functions
    let source_body_ref: &SetExpr = &source_query.body;
    let rows_to_insert = extract_insert_values(source_body_ref)?; // Pass &SetExpr

    for row in rows_to_insert {
        storage_engine.insert_row(&table_name_str, row)
            .map_err(ExecutionError::StorageError)?;
    }
    Ok(())
}

fn extract_table_name(query_body: &SetExpr) -> Result<String, ExecutionError> {
    match query_body {
        SetExpr::Select(select_expr) => {
            if let Some(from_clause) = select_expr.from.get(0) {
                match &from_clause.relation {
                    TableFactor::Table { name, .. } => {
                        if let Some(ident) = name.0.get(0) {
                            Ok(ident.value.clone())
                        } else {
                            Err(ExecutionError::SyntaxError)
                        }
                    }
                    _ => Err(ExecutionError::UnsupportedStatement),
                }
            } else {
                Err(ExecutionError::SyntaxError) // No FROM clause
            }
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn extract_columns_from_select(query_body: &SetExpr) -> Result<Vec<String>, ExecutionError> {
    match query_body {
        SetExpr::Select(select_expr) => {
            Ok(select_expr.projection.iter().map(|item| {
                match item {
                    SelectItem::UnnamedExpr(Expr::Identifier(ident)) => ident.value.clone(),
                    SelectItem::Wildcard => "*".to_string(),
                    // TODO: Handle AliasedExpr, QualifiedWildcard, etc.
                    _ => unimplemented!("Unsupported select item for column extraction"),
                }
            }).collect())
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn extract_condition_from_select(query_body: &SetExpr) -> Result<Option<Condition>, ExecutionError> {
    match query_body {
        SetExpr::Select(select_expr) => {
            match &select_expr.selection {
                Some(expr) => {
                    match expr {
                        Expr::BinaryOp { left, op, right } => {
                            let left_col = match left.as_ref() {
                                Expr::Identifier(ident) => ident.value.clone(),
                                _ => return Err(ExecutionError::UnsupportedStatement),
                            };

                            let value_expr = match right.as_ref() {
                                Expr::Value(v) => v,
                                _ => return Err(ExecutionError::UnsupportedStatement), // Only support direct value comparison for now
                            };

                            let parsed_value = match value_expr {
                                sqlparser::ast::Value::Number(s, _l) => {
                                    s.parse::<i32>().map(Value::Int).map_err(|_| ExecutionError::SyntaxError)?
                                },
                                sqlparser::ast::Value::SingleQuotedString(s) => Value::Varchar(s.clone()),
                                sqlparser::ast::Value::Boolean(b) => Value::Varchar(b.to_string()),
                                // TODO: Handle other sqlparser::ast::Value variants
                                _ => return Err(ExecutionError::UnsupportedStatement),
                            };

                            match op {
                                BinaryOperator::Eq => Ok(Some(Condition::Equals(left_col, parsed_value))),
                                BinaryOperator::Gt => Ok(Some(Condition::GreaterThan(left_col, parsed_value))),
                                BinaryOperator::Lt => Ok(Some(Condition::LessThan(left_col, parsed_value))),
                                // TODO: Handle other operators like Ne, GtEq, LtEq, And, Or
                                _ => Err(ExecutionError::UnsupportedStatement),
                            }
                        }
                        // TODO: Handle other condition expressions
                        _ => Ok(None), // No condition or unsupported condition type
                    }
                }
                None => Ok(None), // No WHERE clause
            }
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn extract_insert_values(source_body: &SetExpr) -> Result<Vec<Row>, ExecutionError> { // Takes &SetExpr
    match source_body { // No need for as_ref() if already &SetExpr
        SetExpr::Values(values_list) => {
            values_list.0.iter().map(|row_exprs| {
                let values_vec: Result<Vec<Value>, ExecutionError> = row_exprs.iter().map(|expr| {
                    match expr {
                        Expr::Value(sqlparser::ast::Value::Number(s, _l)) => {
                            s.parse::<i32>().map(Value::Int).map_err(|_| ExecutionError::SyntaxError)
                        },
                        Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => Ok(Value::Varchar(s.clone())),
                        Expr::Value(sqlparser::ast::Value::Boolean(b)) => Ok(Value::Varchar(b.to_string())), // Storing boolean as Varchar for now, consider dedicated type in Value enum
                        Expr::Value(sqlparser::ast::Value::Null) => Ok(Value::Null),
                        _ => Err(ExecutionError::UnsupportedStatement),
                    }
                }).collect();
                values_vec.map(|values| Row { values })
            }).collect()
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlparser::parser::Parser;
    use sqlparser::dialect::GenericDialect;

    #[derive(Clone)]
    struct MockExecutorStorageEngine {
        tables: std::collections::HashMap<String, (Schema, Vec<Row>)>
    }

    impl MockExecutorStorageEngine {
        fn new() -> Self {
            MockExecutorStorageEngine { tables: std::collections::HashMap::new() }
        }
    }

    impl StorageEngine for MockExecutorStorageEngine {
        fn create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String> {
            if self.tables.contains_key(table_name) {
                return Err(format!("Table {} already exists", table_name));
            }
            self.tables.insert(table_name.to_string(), (schema, Vec::new()));
            Ok(())
        }

        fn drop_table(&mut self, table_name: &str) -> Result<(), String> {
            if self.tables.remove(table_name).is_some() {
                Ok(())
            } else {
                Err(format!("Table {} not found", table_name))
            }
        }

        fn insert_row(&mut self, table_name: &str, row: Row) -> Result<(), String> {
            match self.tables.get_mut(table_name) {
                Some((_schema, rows)) => {
                    rows.push(row);
                    Ok(())
                }
                None => Err(format!("Table {} not found", table_name)),
            }
        }

        fn update_rows(&mut self, _table_name: &str, _updates: Vec<(String, Value)>, _condition: Option<Condition>) -> Result<u64, String> {
            unimplemented!()
        }

        fn delete_rows(&mut self, _table_name: &str, _condition: Option<Condition>) -> Result<u64, String> {
            unimplemented!()
        }

        fn select_rows(&self, table_name: &str, columns: Vec<String>, condition: Option<Condition>) -> Result<Vec<Row>, String> {
            match self.tables.get(table_name) {
                Some((schema, rows)) => {
                    let mut result_rows = Vec::new();
                    for row in rows {
                        if columns.contains(&"*".to_string()) || !columns.is_empty() {
                            let mut projected_row_values = Vec::new();
                            if columns.contains(&"*".to_string()) {
                                projected_row_values = row.values.clone();
                            } else {
                                for col_name in &columns {
                                    if let Some(idx) = schema.columns.iter().position(|c| &c.name == col_name) {
                                        projected_row_values.push(row.values.get(idx).cloned().unwrap_or(Value::Null));
                                    } else {
                                        return Err(format!("Column {} not found in table {}", col_name, table_name));
                                    }
                                }
                            }
                            result_rows.push(Row { values: projected_row_values });
                        }
                    }
                    Ok(result_rows)
                }
                None => Err(format!("Table {} not found", table_name)),
            }
        }

        fn get_table_schema(&self, table_name: &str) -> Result<Schema, String> {
            match self.tables.get(table_name) {
                Some((schema, _)) => Ok(schema.clone()),
                None => Err(format!("Table {} not found", table_name)),
            }
        }
    }

    fn parse_sql_to_statement(sql: &str) -> Statement {
        Parser::parse_sql(&GenericDialect {}, sql).unwrap().remove(0)
    }

    #[test]
    fn test_execute_select_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let table_name = "test_table";
        let schema = Schema {
            columns: vec![
                ColumnDefinition { name: "id".to_string(), data_type: StorageDataType::Int(32), constraints: vec![] },
                ColumnDefinition { name: "name".to_string(), data_type: StorageDataType::Varchar(255), constraints: vec![] },
            ]
        };
        mock_storage.create_table(table_name, schema.clone()).unwrap();
        mock_storage.insert_row(table_name, Row { values: vec![Value::Int(1), Value::Varchar("Alice".to_string())] }).unwrap();

        let sql = "SELECT id, name FROM test_table WHERE id = 1";
        let statement = parse_sql_to_statement(sql);

        match execute_ast(statement, &mut mock_storage) {
            Ok(QueryResult::Data(rows)) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].values[0], Value::Int(1));
                assert_eq!(rows[0].values[1], Value::Varchar("Alice".to_string()));
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type"),
        }
    }

    #[test]
    fn test_execute_insert_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let table_name = "test_table";
        let schema = Schema {
            columns: vec![
                ColumnDefinition { name: "id".to_string(), data_type: StorageDataType::Int(32), constraints: vec![] },
                ColumnDefinition { name: "name".to_string(), data_type: StorageDataType::Varchar(255), constraints: vec![] },
            ]
        };
        mock_storage.create_table(table_name, schema.clone()).unwrap();

        let sql = "INSERT INTO test_table (id, name) VALUES (1, \'Bob\')";
        let statement = parse_sql_to_statement(sql);

        match execute_ast(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage.select_rows(table_name, vec!["id".to_string(), "name".to_string()], None).unwrap();
                assert_eq!(selected_rows.len(), 1);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(selected_rows[0].values[1], Value::Varchar("Bob".to_string()));
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for INSERT"),
        }
    }

    #[test]
    fn test_execute_create_table_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let sql = "CREATE TABLE new_users (id INT, email VARCHAR(100), created_at TIMESTAMP)";
        let statement = parse_sql_to_statement(sql);

        match execute_ast(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let schema_result = mock_storage.get_table_schema("new_users");
                assert!(schema_result.is_ok());
                let schema = schema_result.unwrap();
                assert_eq!(schema.columns.len(), 3);
                assert_eq!(schema.columns[0].name, "id");
                assert_eq!(schema.columns[0].data_type, StorageDataType::Int(32));
                assert_eq!(schema.columns[1].name, "email");
                assert_eq!(schema.columns[1].data_type, StorageDataType::Varchar(255)); // Defaulted in handler
                assert_eq!(schema.columns[2].name, "created_at");
                assert_eq!(schema.columns[2].data_type, StorageDataType::Varchar(29)); // Mapped from Timestamp
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for CREATE TABLE"),
        }
    }

    #[test]
    fn test_extract_condition_less_than() {
        let sql = "SELECT id FROM test_table WHERE age < 25";
        let statement = parse_sql_to_statement(sql);
        match statement {
            Statement::Query(query) => {
                let condition = extract_condition_from_select(&query.body).unwrap();
                match condition {
                    Some(Condition::LessThan(col, val)) => {
                        assert_eq!(col, "age");
                        assert_eq!(val, Value::Int(25));
                    }
                    _ => panic!("Expected LessThan condition"),
                }
            }
            _ => panic!("Expected Query statement"),
        }
    }

    #[test]
    fn test_extract_condition_equals_varchar() {
        let sql = "SELECT id FROM test_table WHERE name = 'Alice'";
        let statement = parse_sql_to_statement(sql);
        match statement {
            Statement::Query(query) => {
                let condition = extract_condition_from_select(&query.body).unwrap();
                match condition {
                    Some(Condition::Equals(col, val)) => {
                        assert_eq!(col, "name");
                        assert_eq!(val, Value::Varchar("Alice".to_string()));
                    }
                    _ => panic!("Expected Equals condition with Varchar"),
                }
            }
            _ => panic!("Expected Query statement"),
        }
    }

    #[test]
    fn test_extract_condition_no_where() {
        let sql = "SELECT id FROM test_table";
        let statement = parse_sql_to_statement(sql);
        match statement {
            Statement::Query(query) => {
                let condition = extract_condition_from_select(&query.body).unwrap();
                assert!(condition.is_none(), "Expected no condition");
            }
            _ => panic!("Expected Query statement"),
        }
    }

    #[test]
    fn test_execute_insert_multiple_rows() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let table_name = "multi_insert_table";
        let schema = Schema {
            columns: vec![
                ColumnDefinition { name: "id".to_string(), data_type: StorageDataType::Int(32), constraints: vec![] },
                ColumnDefinition { name: "item".to_string(), data_type: StorageDataType::Varchar(50), constraints: vec![] },
            ]
        };
        mock_storage.create_table(table_name, schema.clone()).unwrap();

        let sql = "INSERT INTO multi_insert_table (id, item) VALUES (1, 'Apple'), (2, 'Banana'), (3, 'Cherry')";
        let statement = parse_sql_to_statement(sql);

        match execute_ast(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage.select_rows(table_name, vec!["id".to_string(), "item".to_string()], None).unwrap();
                assert_eq!(selected_rows.len(), 3);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(selected_rows[0].values[1], Value::Varchar("Apple".to_string()));
                assert_eq!(selected_rows[1].values[0], Value::Int(2));
                assert_eq!(selected_rows[1].values[1], Value::Varchar("Banana".to_string()));
                assert_eq!(selected_rows[2].values[0], Value::Int(3));
                assert_eq!(selected_rows[2].values[1], Value::Varchar("Cherry".to_string()));
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for INSERT"),
        }
    }

    #[test]
    fn test_execute_insert_null_value() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let table_name = "null_test_table";
        let schema = Schema {
            columns: vec![
                ColumnDefinition { name: "id".to_string(), data_type: StorageDataType::Int(32), constraints: vec![] },
                ColumnDefinition { name: "description".to_string(), data_type: StorageDataType::Varchar(255), constraints: vec![] },
            ]
        };
        mock_storage.create_table(table_name, schema.clone()).unwrap();

        let sql = "INSERT INTO null_test_table (id, description) VALUES (1, NULL)";
        let statement = parse_sql_to_statement(sql);

        match execute_ast(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage.select_rows(table_name, vec!["id".to_string(), "description".to_string()], None).unwrap();
                assert_eq!(selected_rows.len(), 1);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(selected_rows[0].values[1], Value::Null);
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for INSERT"),
        }
    }
}
