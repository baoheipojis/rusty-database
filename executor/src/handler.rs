use crate::error::ExecutionError;
use sqlparser::ast::{
    BinaryOperator, DataType as SQLDataType, Expr, Ident, ObjectName, Query, SelectItem, SetExpr,
    Statement, TableFactor,
}; // Renamed DataType, removed unused Values
use storage::storage_engine_interface::{
    ColumnDefinition, Condition, Constraint, DataType as StorageDataType, Row, Schema, StorageEngine, Value,
}; // Added Constraint import

// Represents the result of a query execution
#[derive(Debug)]
pub enum QueryResult {
    Data(Vec<Row>),
    RowsAffected(u64),
    Success, // For statements like CREATE TABLE, INSERT without returning data
}

impl QueryResult {
    /// Display query results in a formatted table
    pub fn display(&self, column_names: Option<&[String]>) {
        match self {
            QueryResult::Data(rows) => {
                if rows.is_empty() {
                    println!("No rows returned.");
                    return;
                }
                
                let num_cols = rows[0].values.len();
                let mut col_widths = vec![0; num_cols];
                
                // Default column names if not provided
                let default_names: Vec<String> = (0..num_cols)
                    .map(|i| format!("col{}", i + 1))
                    .collect();
                let headers = column_names.unwrap_or(&default_names);
                
                // Calculate width for headers (minimum 3 characters)
                for (i, header) in headers.iter().enumerate() {
                    if i < col_widths.len() {
                        col_widths[i] = header.len().max(3);
                    }
                }
                
                // Calculate width for data values (without quotes for strings)
                for row in rows {
                    for (i, value) in row.values.iter().enumerate() {
                        if i < col_widths.len() {
                            let display_str = match value {
                                Value::Int(n) => n.to_string(),
                                Value::Varchar(s) => s.clone(),
                                Value::Null => "NULL".to_string(),
                            };
                            col_widths[i] = col_widths[i].max(display_str.len());
                        }
                    }
                }
                
                // Print header row with left alignment and proper spacing
                print!("|");
                for (i, header) in headers.iter().enumerate() {
                    if i < col_widths.len() {
                        print!(" {:<width$} |", header, width = col_widths[i]);
                    }
                }
                println!();
                
                // Print separator row
                print!("|");
                for &width in &col_widths {
                    print!(" {} |", "-".repeat(width));
                }
                println!();
                
                // Print data rows with left alignment
                for row in rows {
                    print!("|");
                    for (i, value) in row.values.iter().enumerate() {
                        if i < col_widths.len() {
                            let display_str = match value {
                                Value::Int(n) => n.to_string(),
                                Value::Varchar(s) => s.clone(),
                                Value::Null => "NULL".to_string(),
                            };
                            print!(" {:<width$} |", display_str, width = col_widths[i]);
                        }
                    }
                    println!();
                }
            }
            _ => {
                // Don't print anything for non-SELECT statements
            }
        }
    }
    
    /// Format query results as string for testing
    pub fn format_as_string(&self, column_names: Option<&[String]>) -> String {
        match self {
            QueryResult::Data(rows) => {
                if rows.is_empty() {
                    return "No rows returned.".to_string();
                }
                
                let num_cols = rows[0].values.len();
                let mut col_widths = vec![0; num_cols];
                
                // Default column names if not provided
                let default_names: Vec<String> = (0..num_cols)
                    .map(|i| format!("col{}", i + 1))
                    .collect();
                let headers = column_names.unwrap_or(&default_names);
                
                // Calculate width for headers (minimum 3 characters)
                for (i, header) in headers.iter().enumerate() {
                    if i < col_widths.len() {
                        col_widths[i] = header.len().max(3);
                    }
                }
                
                // Calculate width for data values (without quotes for strings)
                for row in rows {
                    for (i, value) in row.values.iter().enumerate() {
                        if i < col_widths.len() {
                            let display_str = match value {
                                Value::Int(n) => n.to_string(),
                                Value::Varchar(s) => s.clone(),
                                Value::Null => "NULL".to_string(),
                            };
                            col_widths[i] = col_widths[i].max(display_str.len());
                        }
                    }
                }
                
                let mut result = String::new();
                
                // Header row with left alignment and proper spacing
                result.push('|');
                for (i, header) in headers.iter().enumerate() {
                    if i < col_widths.len() {
                        result.push_str(&format!(" {:<width$} |", header, width = col_widths[i]));
                    }
                }
                result.push('\n');
                
                // Separator row
                result.push('|');
                for &width in &col_widths {
                    result.push_str(&format!(" {} |", "-".repeat(width)));
                }
                result.push('\n');
                
                // Data rows with left alignment
                for row in rows {
                    result.push('|');
                    for (i, value) in row.values.iter().enumerate() {
                        if i < col_widths.len() {
                            let display_str = match value {
                                Value::Int(n) => n.to_string(),
                                Value::Varchar(s) => s.clone(),
                                Value::Null => "NULL".to_string(),
                            };
                            result.push_str(&format!(" {:<width$} |", display_str, width = col_widths[i]));
                        }
                    }
                    result.push('\n');
                }
                
                result
            }
            _ => String::new(),
        }
    }
}

pub fn execute_stmt(
    statement: Statement,
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    match statement {
        Statement::Query(query) => handle_query_stmt(&query, storage_engine),
        Statement::Insert {
            table_name,
            columns,
            source,
            ..
        } => handle_insert_stmt(&table_name, &columns, &source, storage_engine),
        Statement::CreateTable {
            name,
            columns: ast_columns,
            ..
        } => handle_create_table_stmt(&name, &ast_columns, storage_engine),
        Statement::Drop {
            object_type,
            if_exists,
            names,
            ..
        } => handle_drop_table_stmt(&object_type, if_exists, &names, storage_engine),
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn handle_query_stmt(
    query: &Query,
    storage_engine: &dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    let rows = handle_query(query, storage_engine)?;
    let result = QueryResult::Data(rows);
    
    // Extract column names from the query for display
    let column_names = extract_columns_from_select(&query.body).ok();
    
    // Print the query results with column names
    result.display(column_names.as_deref());
    
    Ok(result)
}

fn handle_insert_stmt(
    table_name: &ObjectName,
    columns: &[Ident],
    source: &Query,
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    handle_insert(table_name, columns, source, storage_engine)?;
    Ok(QueryResult::Success)
}
/// 创建表
///
/// # 参数
/// - `name`: 表名的 AST 对象
/// - `ast_columns`: 列定义的 AST 数组
/// - `storage_engine`: 存储引擎 trait 对象
///
/// # 返回
/// - `Ok(QueryResult::Success)`：成功
/// - `Err(ExecutionError)`：失败
fn handle_create_table_stmt(
    name: &ObjectName,
    ast_columns: &[sqlparser::ast::ColumnDef],
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    // 解析表名，如果解析失败，整个函数都返回。
    // clone是因为因为我们建表也要拿到字符串的所有权，否则如果AST被释放，表名就没了。
    let table_name_str = name
        .0
        .get(0)
        .ok_or(ExecutionError::SyntaxError)?
        .value
        .clone();
    let mut schema_columns = Vec::new();
    for col_def in ast_columns {
        let column_name = col_def.name.value.clone();
        // 我们只用支持INT和VARCHAR类型
        let data_type = match &col_def.data_type {
            // Int(opt_len)，后面的数字是可选的长度
            SQLDataType::Int(opt_len) => {
                // map的意思是：如果是Some(n)，就把n转换成u32，它的参数是一个函数。
                // unwrap_or是Option的方法，如果是Some(n)，就返回n；如果是None，就返回后面的值。
                let len = opt_len.map(|n| n as u32).unwrap_or(32);
                StorageDataType::Int(len)
            }
            SQLDataType::Varchar(opt_len) => {
                let len = opt_len.map(|n| n as u32).unwrap_or(255);
                StorageDataType::Varchar(len)
            }
            _ => return Err(ExecutionError::UnsupportedStatement),
        };
        // 处理约束 - 支持 NOT NULL 和 PRIMARY KEY
        let constraints = col_def
            .options
            .iter()
            .filter_map(|opt| {
                match &opt.option {
                    sqlparser::ast::ColumnOption::NotNull => {
                        Some(Constraint::NotNull)
                    }
                    sqlparser::ast::ColumnOption::Unique { is_primary } if *is_primary => {
                        // PRIMARY KEY 是使用 Unique 变体并带有 is_primary 标志
                        Some(Constraint::PrimaryKey)
                    }
                    _ => None, // 忽略其他约束类型
                }
            })
            .collect();
        schema_columns.push(ColumnDefinition {
            name: column_name,
            data_type,
            constraints,
        });
    }
    let schema = Schema {
        columns: schema_columns,
    };
    storage_engine
        .create_table(&table_name_str, schema)
        .map_err(ExecutionError::StorageError)?;
    Ok(QueryResult::Success)
}

fn handle_drop_table_stmt(
    object_type: &sqlparser::ast::ObjectType,
    _if_exists: bool,
    names: &[ObjectName],
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    if object_type.to_string().to_uppercase() == "TABLE" {
        let table_name = names
            .get(0)
            .and_then(|n| n.0.get(0))
            .map(|id| id.value.clone())
            .ok_or(ExecutionError::SyntaxError)?;
        storage_engine
            .drop_table(&table_name)
            .map_err(ExecutionError::StorageError)?;
        Ok(QueryResult::Success)
    } else {
        Err(ExecutionError::UnsupportedStatement)
    }
}

fn handle_query(
    query: &Query,
    storage_engine: &dyn StorageEngine,
) -> Result<Vec<Row>, ExecutionError> {
    // Dereference Box<SetExpr> to get &SetExpr for helper functions
    let query_body_ref: &SetExpr = &query.body;
    let table_name = extract_table_name(query_body_ref)?;
    let columns = extract_columns_from_select(query_body_ref)?;
    let condition = extract_condition_from_select(query_body_ref)?;

    storage_engine
        .select_rows(&table_name, columns, condition)
        .map_err(ExecutionError::StorageError)
}

// Removed unused columns_idents parameter
fn handle_insert(
    table_name_obj: &ObjectName,
    _columns_idents: &[Ident],
    source_query: &Query,
    storage_engine: &mut dyn StorageEngine,
) -> Result<(), ExecutionError> {
    let table_name_str = table_name_obj
        .0
        .get(0)
        .ok_or(ExecutionError::SyntaxError)?
        .value
        .clone();

    // Dereference Box<SetExpr> to get &SetExpr for helper functions
    let source_body_ref: &SetExpr = &source_query.body;
    let rows_to_insert = extract_insert_values(source_body_ref)?; // Pass &SetExpr

    for row in rows_to_insert {
        storage_engine
            .insert_row(&table_name_str, row)
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
            Ok(select_expr
                .projection
                .iter()
                .map(|item| {
                    match item {
                        SelectItem::UnnamedExpr(Expr::Identifier(ident)) => ident.value.clone(),
                        SelectItem::Wildcard => "*".to_string(),
                        // TODO: Handle AliasedExpr, QualifiedWildcard, etc.
                        _ => unimplemented!("Unsupported select item for column extraction"),
                    }
                })
                .collect())
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn extract_condition_from_select(
    query_body: &SetExpr,
) -> Result<Option<Condition>, ExecutionError> {
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
                                sqlparser::ast::Value::Number(s, _l) => s
                                    .parse::<i32>()
                                    .map(Value::Int)
                                    .map_err(|_| ExecutionError::SyntaxError)?,
                                sqlparser::ast::Value::SingleQuotedString(s) => {
                                    Value::Varchar(s.clone())
                                }
                                sqlparser::ast::Value::Boolean(b) => Value::Varchar(b.to_string()),
                                // TODO: Handle other sqlparser::ast::Value variants
                                _ => return Err(ExecutionError::UnsupportedStatement),
                            };

                            match op {
                                BinaryOperator::Eq => {
                                    Ok(Some(Condition::Equals(left_col, parsed_value)))
                                }
                                BinaryOperator::Gt => {
                                    Ok(Some(Condition::GreaterThan(left_col, parsed_value)))
                                }
                                BinaryOperator::Lt => {
                                    Ok(Some(Condition::LessThan(left_col, parsed_value)))
                                }
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

fn extract_insert_values(source_body: &SetExpr) -> Result<Vec<Row>, ExecutionError> {
    // Takes &SetExpr
    match source_body {
        // No need for as_ref() if already &SetExpr
        SetExpr::Values(values_list) => {
            values_list
                .0
                .iter()
                .map(|row_exprs| {
                    let values_vec: Result<Vec<Value>, ExecutionError> = row_exprs
                        .iter()
                        .map(|expr| {
                            match expr {
                                Expr::Value(sqlparser::ast::Value::Number(s, _l)) => s
                                    .parse::<i32>()
                                    .map(Value::Int)
                                    .map_err(|_| ExecutionError::SyntaxError),
                                Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => {
                                    Ok(Value::Varchar(s.clone()))
                                }
                                Expr::Value(sqlparser::ast::Value::Boolean(b)) => {
                                    Ok(Value::Varchar(b.to_string()))
                                } // Storing boolean as Varchar for now, consider dedicated type in Value enum
                                Expr::Value(sqlparser::ast::Value::Null) => Ok(Value::Null),
                                _ => Err(ExecutionError::UnsupportedStatement),
                            }
                        })
                        .collect();
                    values_vec.map(|values| Row { values })
                })
                .collect()
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

// cfg是configuration，表示只在test模式下编译。
#[cfg(test)]
pub mod tests {
    // 导入父模块中所有公开的项
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    #[derive(Clone)]
    pub struct MockExecutorStorageEngine {
        tables: std::collections::HashMap<String, (Schema, Vec<Row>)>,
    }

    impl MockExecutorStorageEngine {
        pub fn new() -> Self {
            MockExecutorStorageEngine {
                tables: std::collections::HashMap::new(),
            }
        }
    }

    impl StorageEngine for MockExecutorStorageEngine {
        fn create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String> {
            if self.tables.contains_key(table_name) {
                return Err(format!("Table {} already exists", table_name));
            }
            self.tables
                .insert(table_name.to_string(), (schema, Vec::new()));
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

        fn update_rows(
            &mut self,
            _table_name: &str,
            _updates: Vec<(String, Value)>,
            _condition: Option<Condition>,
        ) -> Result<u64, String> {
            unimplemented!()
        }

        fn delete_rows(
            &mut self,
            _table_name: &str,
            _condition: Option<Condition>,
        ) -> Result<u64, String> {
            unimplemented!()
        }

        fn select_rows(
            &self,
            table_name: &str,
            columns: Vec<String>,
            condition: Option<Condition>,
        ) -> Result<Vec<Row>, String> {
            // Removed _ from condition
            match self.tables.get(table_name) {
                Some((schema, all_rows)) => {
                    let mut filtered_rows = Vec::new();

                    for row in all_rows {
                        let mut matches_condition = true; // Assume true if no condition or if condition is met
                        if let Some(cond) = &condition {
                            // Determine the column name and expected value from the condition
                            let (cond_col_name_str, expected_val_opt, op_type) = match cond {
                                Condition::Equals(name, val) => (name.as_str(), Some(val), "eq"),
                                Condition::GreaterThan(name, val) => {
                                    (name.as_str(), Some(val), "gt")
                                }
                                Condition::LessThan(name, val) => (name.as_str(), Some(val), "lt"),
                                Condition::IsNull(name) => (name.as_str(), None, "isnull"),
                                Condition::IsNotNull(name) => (name.as_str(), None, "isnotnull"),
                                // If other conditions are added to the Condition enum, they'd need handling here
                            };

                            if let Some(col_idx) = schema
                                .columns
                                .iter()
                                .position(|c| c.name == cond_col_name_str)
                            {
                                let actual_value = &row.values[col_idx];

                                matches_condition = match op_type {
                                    "eq" => expected_val_opt.map_or(false, |ev| actual_value == ev),
                                    "gt" => match (actual_value, expected_val_opt) {
                                        (Value::Int(a), Some(Value::Int(e))) => a > e,
                                        (Value::Varchar(a), Some(Value::Varchar(e))) => a > e,
                                        _ => false, // Type mismatch or not comparable for GT
                                    },
                                    "lt" => match (actual_value, expected_val_opt) {
                                        (Value::Int(a), Some(Value::Int(e))) => a < e,
                                        (Value::Varchar(a), Some(Value::Varchar(e))) => a < e,
                                        _ => false, // Type mismatch or not comparable for LT
                                    },
                                    "isnull" => matches!(actual_value, Value::Null),
                                    "isnotnull" => !matches!(actual_value, Value::Null),
                                    _ => false, // Should not happen
                                };
                            } else {
                                return Err(format!(
                                    "Column {} in condition not found in table {}",
                                    cond_col_name_str, table_name
                                ));
                            }
                        }

                        if matches_condition {
                            filtered_rows.push(row.clone());
                        }
                    }

                    // Projection logic (operates on filtered_rows)
                    let mut result_rows = Vec::new();
                    for row_to_project in &filtered_rows {
                        let mut projected_row_values = Vec::new();
                        if columns.contains(&"*".to_string()) || columns.is_empty() {
                            // Treat empty columns list as SELECT *
                            projected_row_values = row_to_project.values.clone();
                        } else {
                            for col_name in &columns {
                                if let Some(idx) =
                                    schema.columns.iter().position(|c| &c.name == col_name)
                                {
                                    projected_row_values.push(
                                        row_to_project
                                            .values
                                            .get(idx)
                                            .cloned()
                                            .unwrap_or(Value::Null),
                                    );
                                } else {
                                    return Err(format!(
                                        "Column {} not found for projection in table {}",
                                        col_name, table_name
                                    ));
                                }
                            }
                        }
                        if !projected_row_values.is_empty() || columns.contains(&"*".to_string()) {
                            // Ensure we add a row if * or specific columns led to values
                            result_rows.push(Row {
                                values: projected_row_values,
                            });
                        } else if columns.is_empty() && schema.columns.is_empty() {
                            // Handle SELECT * from empty schema table
                            result_rows.push(Row { values: vec![] });
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
        Parser::parse_sql(&GenericDialect {}, sql)
            .unwrap()
            .remove(0)
    }

    #[test]
    fn test_execute_select_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let table_name = "test_table";
        let schema = Schema {
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: StorageDataType::Int(32),
                    constraints: vec![],
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: StorageDataType::Varchar(255),
                    constraints: vec![],
                },
            ],
        };
        mock_storage
            .create_table(table_name, schema.clone())
            .unwrap();
        mock_storage
            .insert_row(
                table_name,
                Row {
                    values: vec![Value::Int(1), Value::Varchar("Alice".to_string())],
                },
            )
            .unwrap();

        let sql = "SELECT id, name FROM test_table WHERE id = 1";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
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
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: StorageDataType::Int(32),
                    constraints: vec![],
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    data_type: StorageDataType::Varchar(255),
                    constraints: vec![],
                },
            ],
        };
        mock_storage
            .create_table(table_name, schema.clone())
            .unwrap();

        let sql = "INSERT INTO test_table (id, name) VALUES (1, \'Bob\')";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage
                    .select_rows(table_name, vec!["id".to_string(), "name".to_string()], None)
                    .unwrap();
                assert_eq!(selected_rows.len(), 1);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(
                    selected_rows[0].values[1],
                    Value::Varchar("Bob".to_string())
                );
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for INSERT"),
        }
    }

    #[test]
    fn test_execute_create_table_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        // 只用 INT 和 VARCHAR 类型
        let sql = "CREATE TABLE new_users (id INT, email VARCHAR(100));";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let schema_result = mock_storage.get_table_schema("new_users");
                assert!(schema_result.is_ok());
                let schema = schema_result.unwrap();
                assert_eq!(schema.columns.len(), 2);
                assert_eq!(schema.columns[0].name, "id");
                assert_eq!(schema.columns[0].data_type, StorageDataType::Int(32));
                assert_eq!(schema.columns[1].name, "email");
                assert_eq!(schema.columns[1].data_type, StorageDataType::Varchar(100));
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for CREATE TABLE"),
        }
    }

    #[test]
    fn test_execute_create_table_with_int_length() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试带有指定长度的INT类型
        let sql = "CREATE TABLE int_length_test (
            small_int INT(8), 
            normal_int INT, 
            big_int INT(64),
            name VARCHAR(50)
        );";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let schema_result = mock_storage.get_table_schema("int_length_test");
                assert!(schema_result.is_ok());
                let schema = schema_result.unwrap();
                
                // 检查列的数量
                assert_eq!(schema.columns.len(), 4);
                
                // 检查small_int列 - INT(8)
                assert_eq!(schema.columns[0].name, "small_int");
                assert_eq!(schema.columns[0].data_type, StorageDataType::Int(8));
                
                // 检查normal_int列 - INT (默认长度)
                assert_eq!(schema.columns[1].name, "normal_int");
                assert_eq!(schema.columns[1].data_type, StorageDataType::Int(32)); // 默认32位
                
                // 检查big_int列 - INT(64)
                assert_eq!(schema.columns[2].name, "big_int");
                assert_eq!(schema.columns[2].data_type, StorageDataType::Int(64));
                
                // 检查name列 - VARCHAR(50)
                assert_eq!(schema.columns[3].name, "name");
                assert_eq!(schema.columns[3].data_type, StorageDataType::Varchar(50));
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
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: StorageDataType::Int(32),
                    constraints: vec![],
                },
                ColumnDefinition {
                    name: "item".to_string(),
                    data_type: StorageDataType::Varchar(50),
                    constraints: vec![],
                },
            ],
        };
        mock_storage
            .create_table(table_name, schema.clone())
            .unwrap();

        let sql = "INSERT INTO multi_insert_table (id, item) VALUES (1, 'Apple'), (2, 'Banana'), (3, 'Cherry')";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage
                    .select_rows(table_name, vec!["id".to_string(), "item".to_string()], None)
                    .unwrap();
                assert_eq!(selected_rows.len(), 3);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(
                    selected_rows[0].values[1],
                    Value::Varchar("Apple".to_string())
                );
                assert_eq!(selected_rows[1].values[0], Value::Int(2));
                assert_eq!(
                    selected_rows[1].values[1],
                    Value::Varchar("Banana".to_string())
                );
                assert_eq!(selected_rows[2].values[0], Value::Int(3));
                assert_eq!(
                    selected_rows[2].values[1],
                    Value::Varchar("Cherry".to_string())
                );
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
                ColumnDefinition {
                    name: "id".to_string(),
                    data_type: StorageDataType::Int(32),
                    constraints: vec![],
                },
                ColumnDefinition {
                    name: "description".to_string(),
                    data_type: StorageDataType::Varchar(255),
                    constraints: vec![],
                },
            ],
        };
        mock_storage
            .create_table(table_name, schema.clone())
            .unwrap();

        let sql = "INSERT INTO null_test_table (id, description) VALUES (1, NULL)";
        let statement = parse_sql_to_statement(sql);

        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let selected_rows = mock_storage
                    .select_rows(
                        table_name,
                        vec!["id".to_string(), "description".to_string()],
                        None,
                    )
                    .unwrap();
                assert_eq!(selected_rows.len(), 1);
                assert_eq!(selected_rows[0].values[0], Value::Int(1));
                assert_eq!(selected_rows[0].values[1], Value::Null);
            }
            Err(e) => panic!("Execution failed: {:?}", e),
            _ => panic!("Unexpected query result type for INSERT"),
        }
    }

    #[test]
    fn test_execute_drop_table_statement() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        let create_sql = "CREATE TABLE drop_test (id INT, name VARCHAR(100));";
        let drop_sql = "DROP TABLE drop_test;";
        let create_stmt = parse_sql_to_statement(create_sql);
        let drop_stmt = parse_sql_to_statement(drop_sql);
        // 创建表
        let res = execute_stmt(create_stmt, &mut mock_storage);
        assert!(res.is_ok(), "CREATE TABLE should succeed");
        // 确认表存在
        let schema_res = mock_storage.get_table_schema("drop_test");
        assert!(schema_res.is_ok(), "Table should exist after CREATE TABLE");
        // 删除表
        let res = execute_stmt(drop_stmt, &mut mock_storage);
        assert!(res.is_ok(), "DROP TABLE should succeed");
        // 确认表已删除
        let schema_res = mock_storage.get_table_schema("drop_test");
        assert!(
            schema_res.is_err(),
            "Table should not exist after DROP TABLE"
        );
    }

    #[test]
    fn test_execute_create_table_with_constraints() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        // 测试创建带有 PRIMARY KEY 和 NOT NULL 约束的表
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100) NOT NULL);";
        let statement = parse_sql_to_statement(sql);
        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                let schema_result = mock_storage.get_table_schema("users");
                assert!(schema_result.is_ok());
                let schema = schema_result.unwrap();
                
                // 检查列的数量
                assert_eq!(schema.columns.len(), 2);
                
                // 检查id列的PRIMARY KEY约束
                assert_eq!(schema.columns[0].name, "id");
                assert_eq!(schema.columns[0].data_type, StorageDataType::Int(32));
                assert!(schema.columns[0].constraints.contains(&Constraint::PrimaryKey));
                
                // 检查name列的NOT NULL约束
                assert_eq!(schema.columns[1].name, "name");
                assert_eq!(schema.columns[1].data_type, StorageDataType::Varchar(100));
                assert!(schema.columns[1].constraints.contains(&Constraint::NotNull));
            }
            Ok(QueryResult::Data(rows)) => {
                // 根据需要处理 Data 变体
                panic!("Expected Success but got Data with {} rows", rows.len());
            }
            Ok(QueryResult::RowsAffected(count)) => {
                // 根据需要处理 RowsAffected 变体
                panic!("Expected Success but got RowsAffected with count = {}", count);
            }
            Err(e) => panic!("Execution failed: {:?}", e),
        }
    }

    #[test]
    fn test_insert_after_drop_table() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 1. 创建表
        let create_sql = "CREATE TABLE temp_table (id INT, data VARCHAR(100));";
        let statement = parse_sql_to_statement(create_sql);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_ok(), "CREATE TABLE should succeed");
        
        // 2. 删除表
        let drop_sql = "DROP TABLE temp_table;";
        let statement = parse_sql_to_statement(drop_sql);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_ok(), "DROP TABLE should succeed");
        
        // 3. 尝试插入数据到已删除的表（应该失败）
        let insert_sql = "INSERT INTO temp_table (id, data) VALUES (1, 'test');";
        let statement = parse_sql_to_statement(insert_sql);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_err(), "INSERT into dropped table should fail");
        
        // 检查错误类型是否与预期匹配（StorageError）
        match result {
            Err(ExecutionError::StorageError(_)) => (), // 预期的错误类型
            _ => panic!("Unexpected error type or success"),
        }
    }
    
    #[test]
    fn test_create_table_with_multiple_primary_keys() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 创建一个表，两列都声明为 PRIMARY KEY
        let sql = "CREATE TABLE multiple_pk (id INT PRIMARY KEY, code INT PRIMARY KEY);";
        let statement = parse_sql_to_statement(sql);
        
        match execute_stmt(statement, &mut mock_storage) {
            Ok(QueryResult::Success) => {
                // 检查表结构
                let schema = mock_storage.get_table_schema("multiple_pk").unwrap();
                
                // 检查哪些列被标记为主键
                let primary_keys_count = schema.columns.iter()
                    .filter(|col| col.constraints.contains(&Constraint::PrimaryKey))
                    .count();
                
                // 根据您的实现，可能允许多主键或只保留最后一个
                // 这个断言可以根据实际实现调整
                assert!(primary_keys_count > 0, "至少应该有一个主键");
                println!("表定义允许 {} 个主键列", primary_keys_count);
            }
            Ok(QueryResult::Data(rows)) => {
                panic!("Expected Success but got Data with {} rows", rows.len());
            }
            Ok(QueryResult::RowsAffected(count)) => {
                panic!("Expected Success but got RowsAffected with count = {}", count);
            }
            Err(e) => {
                // 某些数据库实现会拒绝多主键的定义
                // 这种情况下，错误也是合理的
                println!("创建多主键表被拒绝: {:?}", e);
            }
        }
    }

    #[test]
    fn test_sql_with_single_line_comments() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试单行注释 --
        let sql_with_comments = "
            -- 这是一个单行注释
            CREATE TABLE comment_test (
                id INT, -- 这是ID列
                name VARCHAR(100) -- 这是名称列
            );
            -- 插入测试数据
            INSERT INTO comment_test (id, name) VALUES (1, 'Test');
        ";
        
        let statements = sqlparser::parser::Parser::parse_sql(
            &sqlparser::dialect::GenericDialect {}, 
            sql_with_comments
        );
        
        match statements {
            Ok(parsed_statements) => {
                // 执行解析出的语句
                for statement in parsed_statements {
                    let result = execute_stmt(statement, &mut mock_storage);
                    assert!(result.is_ok(), "语句执行应该成功");
                }
                
                // 验证表是否创建成功
                let schema = mock_storage.get_table_schema("comment_test");
                assert!(schema.is_ok(), "表应该创建成功");
                
                // 验证数据是否插入成功
                let rows = mock_storage.select_rows(
                    "comment_test", 
                    vec!["id".to_string(), "name".to_string()], 
                    None
                );
                assert!(rows.is_ok(), "应该能查询到数据");
                let rows = rows.unwrap();
                assert_eq!(rows.len(), 1, "应该有一行数据");
            }
            Err(e) => {
                println!("解析带注释的SQL失败: {:?}", e);
                // 如果解析失败，说明当前解析器不支持注释
                assert!(false, "SQL解析器应该支持单行注释");
            }
        }
    }
    
    #[test]
    fn test_sql_with_multi_line_comments() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试多行注释 /* */
        let sql_with_comments = "
            /* 这是一个多行注释
               可以跨越多行
               用于详细说明 */
            CREATE TABLE multi_comment_test (
                id INT /* 整数ID */, 
                description VARCHAR(200) /* 描述字段 */
            );
            
            /* 插入一些测试数据 */
            INSERT INTO multi_comment_test (id, description) 
            VALUES (1, 'Multi-line comment test');
        ";
        
        let statements = sqlparser::parser::Parser::parse_sql(
            &sqlparser::dialect::GenericDialect {}, 
            sql_with_comments
        );
        
        match statements {
            Ok(parsed_statements) => {
                // 执行解析出的语句
                for statement in parsed_statements {
                    let result = execute_stmt(statement, &mut mock_storage);
                    assert!(result.is_ok(), "语句执行应该成功");
                }
                
                // 验证表是否创建成功
                let schema = mock_storage.get_table_schema("multi_comment_test");
                assert!(schema.is_ok(), "表应该创建成功");
                
                // 验证数据是否插入成功
                let rows = mock_storage.select_rows(
                    "multi_comment_test", 
                    vec!["id".to_string(), "description".to_string()], 
                    None
                );
                assert!(rows.is_ok(), "应该能查询到数据");
                let rows = rows.unwrap();
                assert_eq!(rows.len(), 1, "应该有一行数据");
            }
            Err(e) => {
                println!("解析带多行注释的SQL失败: {:?}", e);
                // 如果解析失败，说明当前解析器不支持多行注释
                assert!(false, "SQL解析器应该支持多行注释");
            }
        }
    }
    
    #[test]
    fn test_sql_with_mixed_comments() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 测试混合注释
        let sql_with_comments = "
            -- 创建混合注释测试表
            CREATE TABLE mixed_comment_test (
                /* 主键字段 */ id INT PRIMARY KEY, -- 唯一标识符
                /* 数据字段 */ data VARCHAR(100) NOT NULL -- 不能为空的数据
            );
            
            -- 插入测试数据
            INSERT INTO mixed_comment_test (id, data) VALUES 
                (1, 'First record'), /* 第一条记录 */
                (2, 'Second record'); -- 第二条记录
        ";
        
        let statements = sqlparser::parser::Parser::parse_sql(
            &sqlparser::dialect::GenericDialect {}, 
            sql_with_comments
        );
        
        match statements {
            Ok(parsed_statements) => {
                // 执行解析出的语句
                for statement in parsed_statements {
                    let result = execute_stmt(statement, &mut mock_storage);
                    assert!(result.is_ok(), "语句执行应该成功: {:?}", result);
                }
                
                // 验证表创建和约束
                let schema = mock_storage.get_table_schema("mixed_comment_test").unwrap();
                assert_eq!(schema.columns.len(), 2);
                assert!(schema.columns[0].constraints.contains(&Constraint::PrimaryKey));
                assert!(schema.columns[1].constraints.contains(&Constraint::NotNull));
                
                // 验证数据插入
                let rows = mock_storage.select_rows(
                    "mixed_comment_test", 
                    vec!["id".to_string(), "data".to_string()], 
                    None
                ).unwrap();
                assert_eq!(rows.len(), 2, "应该有两行数据");
            }
            Err(e) => {
                println!("解析带混合注释的SQL失败: {:?}", e);
                assert!(false, "SQL解析器应该支持混合注释");
            }
        }
    }

    #[test]
    fn test_display_format_matches_expected_automatically() {
        // Test that our output format exactly matches the expected format from the test case
        let rows = vec![
            Row { values: vec![Value::Int(1), Value::Varchar("Science Fiction".to_string())] },
            Row { values: vec![Value::Int(2), Value::Varchar("Action".to_string())] },
        ];
        let result = QueryResult::Data(rows);
        let column_names = vec!["id".to_string(), "name".to_string()];
        
        let actual_output = result.format_as_string(Some(&column_names));
        // Expected output from the test case file - note the left alignment and proper spacing
        let expected_output = "| id  | name            |\n| --- | --------------- |\n| 1   | Science Fiction |\n| 2   | Action          |\n";
        
        println!("Expected output from test case:");
        println!("{}", expected_output);
        println!("Actual output:");
        println!("{}", actual_output);
        
        // Split into lines for easier comparison
        let actual_lines: Vec<&str> = actual_output.lines().collect();
        let expected_lines: Vec<&str> = expected_output.lines().collect();
        
        // Check line by line
        assert_eq!(actual_lines.len(), expected_lines.len(), "Number of output lines should match");
        
        for (i, (actual, expected)) in actual_lines.iter().zip(expected_lines.iter()).enumerate() {
            assert_eq!(actual, expected, "Line {} should match. Expected: '{}', Actual: '{}'", i + 1, expected, actual);
        }
    }

    #[test]
    fn test_display_format_with_different_column_widths() {
        // Test case with varying column widths
        let rows = vec![
            Row { values: vec![Value::Int(1), Value::Varchar("Science Fiction".to_string())] },
            Row { values: vec![Value::Int(2), Value::Varchar("Action".to_string())] },
        ];
        let result = QueryResult::Data(rows);
        let column_names = vec!["id".to_string(), "name".to_string()];
        
        let output = result.format_as_string(Some(&column_names));
        
        // Check that columns are properly formatted with left alignment
        assert!(output.contains("| id  | name            |"), "Header should have correct left-aligned spacing");
        assert!(output.contains("| --- | --------------- |"), "Separator should match column widths");
        assert!(output.contains("| 1   | Science Fiction |"), "Data should be left-aligned");
        assert!(output.contains("| 2   | Action          |"), "Data should be left-aligned");
        
        println!("Output with different column widths:");
        println!("{}", output);
    }

    #[test]
    fn test_display_format_minimum_width() {
        // Test that columns have minimum width of 3 characters
        let rows = vec![
            Row { values: vec![Value::Int(1), Value::Varchar("A".to_string())] },
            Row { values: vec![Value::Int(2), Value::Varchar("B".to_string())] },
        ];
        let result = QueryResult::Data(rows);
        let column_names = vec!["a".to_string(), "b".to_string()];
        
        let output = result.format_as_string(Some(&column_names));
        
        // Both columns should have minimum width of 3
        assert!(output.contains("| a   | b   |"), "Both columns should have minimum width of 3");
        assert!(output.contains("| --- | --- |"), "Separators should be 3 dashes each");
        assert!(output.contains("| 1   | A   |"), "Data should be left-aligned with padding");
        assert!(output.contains("| 2   | B   |"), "Data should be left-aligned with padding");
        
        println!("Output with minimum width:");
        println!("{}", output);
    }
}
