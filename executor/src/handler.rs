// filepath: executor/src/handler.rs
use crate::error::ExecutionError;
use sqlparser::ast::{
    BinaryOperator, DataType as SQLDataType, Expr, Ident, ObjectName, Query, SelectItem, SetExpr,
    Statement, TableFactor,
}; // Renamed DataType, removed unused Values
use storage::storage_engine_interface::{
    ColumnDefinition, Condition, Constraint, DataType as StorageDataType, Row, Schema, StorageEngine, Value,
}; // Added Constraint import
use std::fs;

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
        Statement::Update {
            table,
            assignments,
            selection,
            ..
        } => handle_update_stmt(&table, &assignments, selection.as_ref(), storage_engine),
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

fn handle_query_stmt(
    query: &Query,
    storage_engine: &dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    // Use the centralized query execution with column resolution
    let (rows, _column_names) = execute_query_with_columns(query, storage_engine)?;
    let result = QueryResult::Data(rows);
    
    // Display results in interactive mode (for backwards compatibility)
    // This behavior can be controlled by the caller
    // result.display(Some(&column_names));
    
    Ok(result)
}

/// Centralized query execution with proper column name resolution
/// This function handles both regular table queries and expression queries,
/// ensuring consistent column naming behavior across all execution paths.
fn execute_query_with_columns(
    query: &Query, 
    storage_engine: &dyn StorageEngine
) -> Result<(Vec<Row>, Vec<String>), ExecutionError> {
    // Execute the query to get rows
    let rows = handle_query(query, storage_engine)?;
    
    if rows.is_empty() {
        // For empty results, return empty rows with default column names
        let default_columns = extract_columns_from_select(&query.body)
            .unwrap_or_else(|_| vec!["col1".to_string()]);
        return Ok((rows, default_columns));
    }
    
    // Extract column names from the query for display
    let mut column_names = extract_columns_from_select(&query.body).ok();
    
    // If this is a SELECT * query, get the actual column names from table schema
    if let Some(ref names) = column_names {
        if names.len() == 1 && names[0] == "*" {
            // This is a SELECT * query, get actual column names from table schema
            if let Ok(table_name) = extract_table_name(&query.body) {
                if let Ok(schema) = storage_engine.get_table_schema(&table_name) {
                    let actual_column_names: Vec<String> = schema.columns.iter()
                        .map(|col| col.name.clone())
                        .collect();
                    column_names = Some(actual_column_names);
                }
            }
        }
    }
    
    // Use the extracted column names or generate default ones
    let cols = column_names.unwrap_or_else(|| {
        // Generate default column names based on number of columns
        let num_cols = if rows.is_empty() { 1 } else { rows[0].values.len() };
        (1..=num_cols).map(|i| format!("col{}", i)).collect()
    });
    Ok((rows, cols))
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
    // storage_engine.create_table() 返回 Result<(), String>
    // 但是这个函数需要返回 Result<QueryResult, ExecutionError>
    // 所以需要把 String 错误转换成 ExecutionError
    // 注意由于后面有个?，所以如果出错了，就会直接返回错误，不会走到后面的Ok了。
    storage_engine
        .create_table(&table_name_str, schema)
        .map_err(ExecutionError::StorageError)?; // 把 String 转换成 ExecutionError::StorageError
    
    // 等价于以下写法：
    // match storage_engine.create_table(&table_name_str, schema) {
    //     Ok(()) => Ok(QueryResult::Success),
    //     Err(storage_error) => Err(ExecutionError::StorageError(storage_error)),
    // }
    
    Ok(QueryResult::Success)
}

fn handle_drop_table_stmt(
    object_type: &sqlparser::ast::ObjectType,
    _if_exists: bool,
    names: &[ObjectName],
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    if object_type.to_string().to_uppercase() == "TABLE" {
        // Drop all tables in the names array
        for name in names {
            let table_name = name
                .0
                .get(0)
                .map(|id| id.value.clone())
                .ok_or(ExecutionError::SyntaxError)?;
            storage_engine
                .drop_table(&table_name)
                .map_err(ExecutionError::StorageError)?;
        }
        Ok(QueryResult::Success)
    } else {
        Err(ExecutionError::UnsupportedStatement)
    }
}

fn handle_update_stmt(
    table: &sqlparser::ast::TableWithJoins,
    assignments: &[sqlparser::ast::Assignment],
    selection: Option<&Expr>,
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError> {
    // Extract table name from TableWithJoins
    let table_name = match &table.relation {
        TableFactor::Table { name, .. } => {
            name.0.get(0)
                .map(|ident| ident.value.clone())
                .ok_or(ExecutionError::SyntaxError)?
        }
        _ => return Err(ExecutionError::UnsupportedStatement),
    };

    // Extract assignments (SET clauses)
    let mut updates = Vec::new();
    for assignment in assignments {
        // assignment.id is a Vec<Ident>, so we need to take the first element
        let column_name = if let Some(ident) = assignment.id.first() {
            ident.value.clone()
        } else {
            return Err(ExecutionError::SyntaxError);
        };
        
        let new_value = match &assignment.value {
            Expr::Value(sqlparser::ast::Value::Number(s, _)) => {
                s.parse::<i32>()
                    .map(Value::Int)
                    .map_err(|_| ExecutionError::SyntaxError)?
            }
            Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => {
                Value::Varchar(s.clone())
            }
            Expr::Value(sqlparser::ast::Value::DoubleQuotedString(s)) => {
                Value::Varchar(s.clone())
            }
            Expr::Value(sqlparser::ast::Value::Null) => Value::Null,
            _ => return Err(ExecutionError::UnsupportedStatement),
        };
        
        updates.push((column_name, new_value));
    }

    // Extract WHERE condition if present
    let condition = if let Some(where_expr) = selection {
        match where_expr {
            Expr::BinaryOp { left, op, right } => {
                let left_col = match left.as_ref() {
                    Expr::Identifier(ident) => ident.value.clone(),
                    _ => return Err(ExecutionError::UnsupportedStatement),
                };

                let value_expr = match right.as_ref() {
                    Expr::Value(v) => v,
                    _ => return Err(ExecutionError::UnsupportedStatement),
                };

                let parsed_value = match value_expr {
                    sqlparser::ast::Value::Number(s, _) => s
                        .parse::<i32>()
                        .map(Value::Int)
                        .map_err(|_| ExecutionError::SyntaxError)?,
                    sqlparser::ast::Value::SingleQuotedString(s) => {
                        Value::Varchar(s.clone())
                    }
                    sqlparser::ast::Value::DoubleQuotedString(s) => {
                        Value::Varchar(s.clone())
                    }
                    sqlparser::ast::Value::Boolean(b) => Value::Varchar(b.to_string()),
                    _ => return Err(ExecutionError::UnsupportedStatement),
                };

                match op {
                    BinaryOperator::Eq => Some(Condition::Equals(left_col, parsed_value)),
                    BinaryOperator::Gt => Some(Condition::GreaterThan(left_col, parsed_value)),
                    BinaryOperator::Lt => Some(Condition::LessThan(left_col, parsed_value)),
                    _ => return Err(ExecutionError::UnsupportedStatement),
                }
            }
            _ => return Err(ExecutionError::UnsupportedStatement),
        }
    } else {
        None
    };

    // Execute the update
    let rows_affected = storage_engine
        .update_rows(&table_name, updates, condition)
        .map_err(ExecutionError::StorageError)?;
    
    Ok(QueryResult::RowsAffected(rows_affected))
}

fn handle_query(
    query: &Query,
    storage_engine: &dyn StorageEngine,
) -> Result<Vec<Row>, ExecutionError> {
    // Dereference Box<SetExpr> to get &SetExpr for helper functions
    let query_body_ref: &SetExpr = &query.body;
    
    // Check if this is a SELECT without FROM clause (expression evaluation)
    if let SetExpr::Select(select_expr) = query_body_ref {
        if select_expr.from.is_empty() {
            // This is a SELECT expression without FROM clause, e.g., SELECT 1 * 2
            return handle_expression_query(select_expr);
        }
    }
    
    // Regular table query
    let table_name = extract_table_name(query_body_ref)?;
    let columns = extract_columns_from_select(query_body_ref)?;
    let condition = extract_condition_from_select(query_body_ref)?;

    storage_engine
        .select_rows(&table_name, columns, condition)
        .map_err(ExecutionError::StorageError)
}

/// Handle SELECT queries without FROM clause (expression evaluation)
fn handle_expression_query(select_expr: &Box<sqlparser::ast::Select>) -> Result<Vec<Row>, ExecutionError> {
    let mut row_values = Vec::new();
    
    for item in &select_expr.projection {
        match item {
            SelectItem::UnnamedExpr(expr) => {
                let value = evaluate_expression(expr)?;
                row_values.push(value);
            }
            _ => return Err(ExecutionError::UnsupportedStatement),
        }
    }
    
    // Return a single row with the calculated values
    Ok(vec![Row { values: row_values }])
}

/// Evaluate an expression to get its value
fn evaluate_expression(expr: &Expr) -> Result<Value, ExecutionError> {
    match expr {
        Expr::Value(sqlparser::ast::Value::Number(s, _)) => {
            s.parse::<i32>()
                .map(Value::Int)
                .map_err(|_| ExecutionError::SyntaxError)
        }
        Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => {
            Ok(Value::Varchar(s.clone()))
        }
        Expr::Value(sqlparser::ast::Value::DoubleQuotedString(s)) => {
            Ok(Value::Varchar(s.clone()))
        }
        Expr::BinaryOp { left, op, right } => {
            let left_val = evaluate_expression(left)?;
            let right_val = evaluate_expression(right)?;
            
            match (left_val, right_val) {
                (Value::Int(a), Value::Int(b)) => {
                    match op {
                        BinaryOperator::Plus => Ok(Value::Int(a + b)),
                        BinaryOperator::Minus => Ok(Value::Int(a - b)),
                        BinaryOperator::Multiply => Ok(Value::Int(a * b)),
                        BinaryOperator::Divide => {
                            if b == 0 {
                                Err(ExecutionError::SyntaxError) // Division by zero
                            } else {
                                Ok(Value::Int(a / b))
                            }
                        }
                        _ => Err(ExecutionError::UnsupportedStatement),
                    }
                }
                _ => Err(ExecutionError::UnsupportedStatement),
            }
        }
        _ => Err(ExecutionError::UnsupportedStatement),
    }
}

// Removed unused columns_idents parameter
fn handle_insert(
    table_name_obj: &ObjectName,
    columns_idents: &[Ident], // 详细解释这个类型
    source_query: &Query,
    storage_engine: &mut dyn StorageEngine,
) -> Result<(), ExecutionError> {
    // 类型解释：
    // &[Ident] 是什么？
    // - & 表示借用（引用）
    // - [] 表示切片（slice）
    // - Ident 是 sqlparser 库中的标识符类型
    // 
    // 完整含义：一个指向 Ident 元素切片的不可变引用
    // 
    // 具体来说：
    // - 当SQL是 INSERT INTO table (col1, col2, col3) VALUES (...)
    // - columns_idents 就包含 [col1, col2, col3] 这些列名的标识符
    
    let table_name_str = table_name_obj
        .0
        .get(0)
        .ok_or(ExecutionError::SyntaxError)?
        .value
        .clone();

    // 获取表的schema来了解所有列
    let table_schema = storage_engine
        .get_table_schema(&table_name_str)
        .map_err(ExecutionError::StorageError)?;

    // 提取指定的列名 - 演示如何使用 &[Ident]
    let specified_columns: Vec<String> = columns_idents
        .iter()              // 迭代切片中的每个 &Ident
        .map(|ident| ident.value.clone())  // 提取每个标识符的字符串值
        .collect();          // 收集成 Vec<String>

    // source_query.body 包含了 INSERT 语句中的 VALUES 部分
    let source_body_ref: &SetExpr = &source_query.body;
    
    // extract_insert_values 会解析 VALUES 部分，提取出实际的行数据
    let partial_rows = extract_insert_values(source_body_ref)?;

    for partial_row in partial_rows {
        let complete_values = if specified_columns.is_empty() {
            // Case: INSERT INTO table VALUES (...) - no columns specified
            // Treat it as inserting into all columns in order
            if partial_row.values.len() != table_schema.columns.len() {
                return Err(ExecutionError::StorageError(format!(
                    "Error: Number of values ({}) doesn't match number of table columns ({})",
                    partial_row.values.len(),
                    table_schema.columns.len()
                )));
            }
            partial_row.values.clone()
        } else {
            // Case: INSERT INTO table (col1, col2, ...) VALUES (...) - specific columns
            if partial_row.values.len() != specified_columns.len() {
                return Err(ExecutionError::StorageError(format!(
                    "Error: Number of values ({}) doesn't match number of specified columns ({})",
                    partial_row.values.len(),
                    specified_columns.len()
                )));
            }

            // 创建完整的行，为所有列分配值
            let mut complete_values = vec![Value::Null; table_schema.columns.len()];
            
            // 将指定列的值填入对应位置
            for (i, column_name) in specified_columns.iter().enumerate() {
                // 找到这个列在表schema中的位置
                let schema_column_index = table_schema
                    .columns
                    .iter()
                    .position(|col| &col.name == column_name)
                    .ok_or_else(|| ExecutionError::StorageError(format!(
                        "Error: Column '{}' does not exist in table '{}'",
                        column_name, table_name_str
                    )))?;
                
                // 将值放到正确的位置
                complete_values[schema_column_index] = partial_row.values[i].clone();
            }
            complete_values
        };

        // 检查约束（在存储之前）
        for (col_idx, column) in table_schema.columns.iter().enumerate() {
            let value = &complete_values[col_idx];
            
            // 检查 NOT NULL 约束
            if column.constraints.contains(&Constraint::NotNull) {
                if matches!(value, Value::Null) {
                    return Err(ExecutionError::StorageError(format!(
                        "NOT NULL constraint violation: column '{}' cannot be NULL. You must provide a value for this column.",
                        column.name
                    )));
                }
            }
        }

        // 创建完整的行并插入
        let complete_row = Row { values: complete_values };
        storage_engine
            .insert_row(&table_name_str, complete_row)
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
                        SelectItem::UnnamedExpr(expr) => {
                            // For expressions like 1 * 2, generate a column name based on the expression
                            format_expression_as_column_name(expr)
                        },
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

/// 从 INSERT 语句的 VALUES 部分提取行数据
/// 
/// # 参数
/// - `source_body`: SQL 解析后的 VALUES 表达式
/// 
/// # 返回值
/// - `Ok(Vec<Row>)`: 解析成功，返回行数据列表
/// - `Err(ExecutionError)`: 解析失败或不支持的语法
/// 
/// # 示例
/// ```rust
/// use sqlparser::ast::SetExpr;
/// 
/// // 解析 INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob')
/// let rows = extract_insert_values(&values_expr)?;
/// // 返回: Vec<Row> 包含两行数据
/// // Row 1: [Value::Int(1), Value::Varchar("Alice".to_string())]
/// // Row 2: [Value::Int(2), Value::Varchar("Bob".to_string())]
/// ```
fn extract_insert_values(source_body: &SetExpr) -> Result<Vec<Row>, ExecutionError> {
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
                                Expr::Value(sqlparser::ast::Value::DoubleQuotedString(s)) => {
                                    Ok(Value::Varchar(s.clone()))
                                }
                                Expr::Value(sqlparser::ast::Value::Boolean(b)) => {
                                    Ok(Value::Varchar(b.to_string()))
                                } // Storing boolean as Varchar for now, consider dedicated type in Value enum
                                Expr::Value(sqlparser::ast::Value::Null) => Ok(Value::Null),
                                // Handle identifiers with quote styles (double-quoted strings parsed as identifiers)
                                Expr::Identifier(ident) if ident.quote_style.is_some() => {
                                    Ok(Value::Varchar(ident.value.clone()))
                                }
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



/// Format an expression as a column name for display
fn format_expression_as_column_name(expr: &Expr) -> String {
    match expr {
        Expr::Value(sqlparser::ast::Value::Number(s, _)) => s.clone(),
        Expr::Value(sqlparser::ast::Value::SingleQuotedString(s)) => s.clone(),
        Expr::Value(sqlparser::ast::Value::DoubleQuotedString(s)) => s.clone(),
        Expr::BinaryOp { left, op, right } => {
            let left_str = format_expression_as_column_name(left);
            let right_str = format_expression_as_column_name(right);
            let op_str = match op {
                BinaryOperator::Plus => "+",
                BinaryOperator::Minus => "-",
                BinaryOperator::Multiply => "*",
                BinaryOperator::Divide => "/",
                _ => "?",
            };
            format!("{} {} {}", left_str, op_str, right_str)
        }
        Expr::Identifier(ident) => ident.value.clone(),
        _ => "expr".to_string(),
    }
}

/// 读取测试用例输入文件
/// 
/// # 参数
/// * `case_number` - 测试用例编号 (1, 2, 3, ...)
/// 
/// # 返回值
/// * `Result<String, String>` - 成功时返回文件内容，失败时返回错误信息
pub fn read_test_case_input(case_number: u32) -> Result<String, String> {
    // 使用 CARGO_MANIFEST_DIR 获取 executor 包的目录，然后向上一级到项目根目录
    let executor_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(executor_dir).parent().unwrap();
    let test_cases_dir = project_root.join("公开测试用例");
    let input_file = test_cases_dir.join(case_number.to_string()).join("input.txt");
    
    fs::read_to_string(&input_file)
        .map_err(|e| format!("Failed to read input file {}: {}", input_file.display(), e))
}

/// 读取测试用例期望输出文件
/// 
/// # 参数
/// * `case_number` - 测试用例编号 (1, 2, 3, ...)
/// 
/// # 返回值
/// * `Result<String, String>` - 成功时返回文件内容，失败时返回错误信息
pub fn read_test_case_output(case_number: u32) -> Result<String, String> {
    // 使用 CARGO_MANIFEST_DIR 获取 executor 包的目录，然后向上一级到项目根目录
    let executor_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(executor_dir).parent().unwrap();
    let test_cases_dir = project_root.join("公开测试用例");
    let output_file = test_cases_dir.join(case_number.to_string()).join("output.txt");
    
    fs::read_to_string(&output_file)
        .map_err(|e| format!("Failed to read output file {}: {}", output_file.display(), e))
}

/// 解析多个SQL语句
/// 
/// # 参数
/// * `sql` - 包含多个SQL语句的字符串
/// 
/// # 返回值
/// * `Result<Vec<Statement>, String>` - 成功时返回语句列表，失败时返回错误信息
pub fn parse_multiple_sql_statements(sql: &str) -> Result<Vec<Statement>, String> {
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;
    
    let dialect = GenericDialect {};
    let mut statements = Vec::new();
    
    // 简单地按分号分割，然后逐个解析
    let lines: Vec<&str> = sql.lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with("--"))
        .collect();
    
    let mut current_statement = String::new();
    
    for line in lines {
        current_statement.push_str(line);
        current_statement.push(' ');
        
        if line.trim().ends_with(';') {
            let stmt_sql = current_statement.trim();
            if !stmt_sql.is_empty() {
                match Parser::parse_sql(&dialect, stmt_sql) {
                    Ok(mut parsed) => {
                        if let Some(statement) = parsed.pop() {
                            statements.push(statement);
                        }
                    }
                    Err(e) => return Err(format!("Parse error for '{}': {}", stmt_sql, e)),
                }
            }
            current_statement.clear();
        }
    }
    
    Ok(statements)
}

/// 执行SQL语句并返回输出结果
/// 
/// # 参数
/// * `input_sql` - 输入的SQL语句字符串
/// * `storage_engine` - 存储引擎实现
/// 
/// # 返回值
/// * `Result<String, String>` - 成功时返回查询输出结果，失败时返回错误信息
pub fn execute_sql_and_get_output<T: StorageEngine>(
    input_sql: &str,
    storage_engine: &mut T
) -> Result<String, String> {
    let mut all_outputs = Vec::new();
    
    // 分割SQL语句并执行
    let statements = parse_multiple_sql_statements(input_sql)?;
    
    for statement in statements {
        // Handle SELECT queries specially to get proper column names
        if let Statement::Query(ref query) = statement {
            // Use the centralized query execution with column resolution
            let (rows, column_names) = execute_query_with_columns(query, storage_engine)
                .map_err(|e| format!("Execution error: {:?}", e))?;
            
            if !rows.is_empty() {
                let query_result = QueryResult::Data(rows);
                let output = query_result.format_as_string(Some(&column_names));
                all_outputs.push(output);
            }
        } else {
            // For non-SELECT statements, just execute them
            let _result = execute_stmt(statement, storage_engine)
                .map_err(|e| format!("Execution error: {:?}", e))?;
            // Non-SELECT statements don't produce output for display
        }
    }
    
    // 连接所有输出，用换行符分隔
    Ok(all_outputs.join("\n"))
}

/// 执行测试用例
/// 
/// # 参数
/// * `case_number` - 测试用例编号
/// * `storage_engine` - 存储引擎实现
/// 
/// # 返回值
/// * `Result<(), String>` - 成功时返回Ok(())，失败时返回错误信息
pub fn run_test_case_with_storage<T: StorageEngine>(
    case_number: u32, 
    storage_engine: &mut T
) -> Result<(), String> {
    let input_sql = read_test_case_input(case_number)?;
    let expected_output = read_test_case_output(case_number)?;
    
    // 执行SQL并获取输出
    let actual_output = execute_sql_and_get_output(&input_sql, storage_engine)?;
    
    // 比较输出，忽略换行符差异
    let expected_normalized = expected_output.trim().replace("\r\n", "\n");
    let actual_normalized = actual_output.trim().replace("\r\n", "\n");
    
    if expected_normalized == actual_normalized {
        println!("测试用例 {} 通过！", case_number);
        println!("实际输出内容:");
        println!("{}", actual_normalized);
        Ok(())
    } else {
        Err(format!(
            "测试用例 {} 失败！\n期望输出:\n{}\n实际输出:\n{}", 
            case_number, expected_normalized, actual_normalized
        ))
    }
}

// cfg是configuration，表示只在test模式下编译。
#[cfg(test)]
pub mod tests {
    // 导入父模块中所有公开的项
    use super::*;
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    /// 执行测试用例（测试模块专用，使用Mock存储引擎）
    /// 
    /// # 参数
    /// * `case_number` - 测试用例编号
    /// 
    /// # 返回值
    /// * `Result<(), String>` - 成功时返回Ok(())，失败时返回错误信息
    pub fn run_test_case(case_number: u32) -> Result<(), String> {
        let mut mock_storage = MockExecutorStorageEngine::new();
        run_test_case_with_storage(case_number, &mut mock_storage)
    }

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
            table_name: &str,
            updates: Vec<(String, Value)>,
            condition: Option<Condition>,
        ) -> Result<u64, String> {
            match self.tables.get_mut(table_name) {
                Some((schema, rows)) => {
                    let mut rows_affected = 0u64;
                    
                    for row in rows.iter_mut() {
                        // Check if row matches condition
                        let matches_condition = if let Some(cond) = &condition {
                            match cond {
                                Condition::Equals(col_name, expected_val) => {
                                    if let Some(col_idx) = schema.columns.iter().position(|c| &c.name == col_name) {
                                        &row.values[col_idx] == expected_val
                                    } else {
                                        false
                                    }
                                }
                                Condition::GreaterThan(col_name, expected_val) => {
                                    if let Some(col_idx) = schema.columns.iter().position(|c| &c.name == col_name) {
                                        match (&row.values[col_idx], expected_val) {
                                            (Value::Int(a), Value::Int(e)) => a > e,
                                            (Value::Varchar(a), Value::Varchar(e)) => a > e,
                                            _ => false,
                                        }
                                    } else {
                                        false
                                    }
                                }
                                Condition::LessThan(col_name, expected_val) => {
                                    if let Some(col_idx) = schema.columns.iter().position(|c| &c.name == col_name) {
                                        match (&row.values[col_idx], expected_val) {
                                            (Value::Int(a), Value::Int(e)) => a < e,
                                            (Value::Varchar(a), Value::Varchar(e)) => a < e,
                                            _ => false,
                                        }
                                    } else {
                                        false
                                    }
                                }
                                _ => false,
                            }
                        } else {
                            true // No condition means update all rows
                        };
                        
                        if matches_condition {
                            // Apply updates to this row
                            for (update_col_name, new_value) in &updates {
                                if let Some(col_idx) = schema.columns.iter().position(|c| &c.name == update_col_name) {
                                    row.values[col_idx] = new_value.clone();
                                }
                            }
                            rows_affected += 1;
                        }
                    }
                    
                    Ok(rows_affected)
                }
                None => Err(format!("Table {} not found", table_name)),
            }
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

    #[test]
    fn test_map_err_explanation() {
        // 演示 map_err 的用法
        
        // 假设我们有一个返回 Result<i32, String> 的函数
        fn divide(a: i32, b: i32) -> Result<i32, String> {
            if b == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(a / b)
            }
        }
        
        // 定义我们自己的错误类型
        #[derive(Debug, PartialEq)]
        enum MyError {
            MathError(String),
            Other,
        }
        
        // 使用 map_err 转换错误类型
        let result: Result<i32, MyError> = divide(10, 2)
            .map_err(MyError::MathError); // 把 String 转换成 MyError::MathError
        
        assert_eq!(result, Ok(5));
        
        let error_result: Result<i32, MyError> = divide(10, 0)
            .map_err(MyError::MathError); // 把 String 转换成 MyError::MathError
        
        assert_eq!(error_result, Err(MyError::MathError("Division by zero".to_string())));
        
        // map_err 等价于：
        let manual_result: Result<i32, MyError> = match divide(10, 0) {
            Ok(value) => Ok(value),
            Err(string_error) => Err(MyError::MathError(string_error)),
        };
        
        assert_eq!(manual_result, Err(MyError::MathError("Division by zero".to_string())));
        
        println!("map_err 测试通过！");
    }

    #[test]
    fn test_public_case_1() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 执行公开测试用例1的SQL序列
        
        // 1. 创建表
        let create_sql = "CREATE TABLE genres (
            id INT PRIMARY KEY,
            name VARCHAR(100) NOT NULL
        );";
        let statement = parse_sql_to_statement(create_sql);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_ok(), "CREATE TABLE should succeed");
        
        // 2. 插入数据 - 第一条记录
        let insert_sql1 = "INSERT INTO genres VALUES (1, \"Science Fiction\");";
        let statement = parse_sql_to_statement(insert_sql1);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_ok(), "First INSERT should succeed: {:?}", result.err());
        
        // 3. 插入数据 - 第二条记录
        let insert_sql2 = "INSERT INTO genres VALUES (2, \"Action\");";
        let statement = parse_sql_to_statement(insert_sql2);
        let result = execute_stmt(statement, &mut mock_storage);
        assert!(result.is_ok(), "Second INSERT should succeed");
        
        // 4. 查询所有数据
        let select_sql = "SELECT * FROM genres;";
        let statement = parse_sql_to_statement(select_sql);
        let result = execute_stmt(statement, &mut mock_storage);
        
        // 验证查询结果
        match result {
            Ok(QueryResult::Data(rows)) => {
                assert_eq!(rows.len(), 2, "Should have 2 rows");
                
                // 验证第一行数据
                assert_eq!(rows[0].values[0], Value::Int(1));
                assert_eq!(rows[0].values[1], Value::Varchar("Science Fiction".to_string()));
                
                // 验证第二行数据
                assert_eq!(rows[1].values[0], Value::Int(2));
                assert_eq!(rows[1].values[1], Value::Varchar("Action".to_string()));
                
                // 验证输出格式与期望的完全匹配
                let query_result = QueryResult::Data(rows);
                let column_names = vec!["id".to_string(), "name".to_string()];
                let actual_output = query_result.format_as_string(Some(&column_names));
                
                // 期望的输出格式（来自output.txt）
                let expected_output = "| id  | name            |\n| --- | --------------- |\n| 1   | Science Fiction |\n| 2   | Action          |\n";
                
                assert_eq!(actual_output, expected_output, 
                    "Output format should match expected format exactly.\nExpected:\n{}\nActual:\n{}", 
                    expected_output, actual_output);
            }
            Ok(_) => panic!("Expected Data result for SELECT query"),
            Err(e) => panic!("SELECT query failed: {:?}", e),
        }
        
        println!("公开测试用例1通过！");
    }

    #[test]
    fn test_public_case_2() {
        let mut mock_storage = MockExecutorStorageEngine::new();
        
        // 读取测试用例2的SQL输入
        let input_sql = read_test_case_input(2).expect("Should be able to read test case 2 input");
        let expected_output = read_test_case_output(2).expect("Should be able to read test case 2 output");
        
        println!("测试用例2输入SQL:");
        println!("{}", input_sql);
        println!("期望输出:");
        println!("{}", expected_output);
        
        // 解析并执行所有SQL语句
        let statements = parse_multiple_sql_statements(&input_sql).expect("Should parse SQL statements");
        
        let mut actual_output = String::new();
        
        for statement in statements {
            println!("执行语句: {:?}", statement);
            let result = execute_stmt(statement, &mut mock_storage);
            
            match result {
                Ok(QueryResult::Data(rows)) => {
                    // 这是SELECT查询的结果
                    let column_names = vec!["id".to_string(), "name".to_string()];
                    let query_result = QueryResult::Data(rows);
                    actual_output = query_result.format_as_string(Some(&column_names));
                    println!("查询结果:");
                    println!("{}", actual_output);
                }
                Ok(QueryResult::Success) => {
                    println!("语句执行成功");
                }
                Ok(QueryResult::RowsAffected(count)) => {
                    println!("影响了 {} 行", count);
                }
                Err(e) => {
                    println!("执行错误: {:?}", e);
                    // 对于某些操作（如DROP不存在的表），可能是预期的错误
                }
            }
        }
        
        // 比较输出，忽略换行符差异
        let expected_normalized = expected_output.trim().replace("\r\n", "\n");
        let actual_normalized = actual_output.trim().replace("\r\n", "\n");
        
        assert_eq!(expected_normalized, actual_normalized, 
            "输出应该匹配期望结果.\n期望:\n{}\n实际:\n{}", 
            expected_normalized, actual_normalized);
            
        println!("公开测试用例2通过！");
    }

    #[test]
    fn test_run_test_case_1() {
        // 使用通用测试函数测试用例1
        match run_test_case(1) {
            Ok(_) => println!("测试用例1通过！"),
            Err(e) => panic!("测试用例1失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_2() {
        // 使用通用测试函数测试用例2
        match run_test_case(2) {
            Ok(_) => println!("测试用例2通过！"),
            Err(e) => panic!("测试用例2失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_3() {
        // 使用通用测试函数测试用例3
        match run_test_case(3) {
            Ok(_) => println!("测试用例3通过！"),
            Err(e) => panic!("测试用例3失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_4() {
        // 使用通用测试函数测试用例4
        match run_test_case(4) {
            Ok(_) => println!("测试用例4通过！"),
            Err(e) => panic!("测试用例4失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_5() {
        // 使用通用测试函数测试用例5
        match run_test_case(5) {
            Ok(_) => println!("测试用例5通过！"),
            Err(e) => panic!("测试用例5失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_6() {
        // 使用通用测试函数测试用例6
        match run_test_case(6) {
            Ok(_) => println!("测试用例6通过！"),
            Err(e) => panic!("测试用例6失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_7() {
        // 使用通用测试函数测试用例7
        match run_test_case(7) {
            Ok(_) => println!("测试用例7通过！"),
            Err(e) => panic!("测试用例7失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_8() {
        // 使用通用测试函数测试用例8
        match run_test_case(8) {
            Ok(_) => println!("测试用例8通过！"),
            Err(e) => panic!("测试用例8失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_10() {
        // 使用通用测试函数测试用例10
        match run_test_case(10) {
            Ok(_) => println!("测试用例10通过！"),
            Err(e) => panic!("测试用例10失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_11() {
        // 使用通用测试函数测试用例11
        match run_test_case(11) {
            Ok(_) => println!("测试用例11通过！"),
            Err(e) => panic!("测试用例11失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_12() {
        // 使用通用测试函数测试用例12
        match run_test_case(12) {
            Ok(_) => println!("测试用例12通过！"),
            Err(e) => panic!("测试用例12失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_13() {
        // 使用通用测试函数测试用例13
        match run_test_case(13) {
            Ok(_) => println!("测试用例13通过！"),
            Err(e) => panic!("测试用例13失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_14() {
        // 使用通用测试函数测试用例14
        match run_test_case(14) {
            Ok(_) => println!("测试用例14通过！"),
            Err(e) => panic!("测试用例14失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_15() {
        // 使用通用测试函数测试用例15
        match run_test_case(15) {
            Ok(_) => println!("测试用例15通过！"),
            Err(e) => panic!("测试用例15失败: {}", e),
        }
    }

    #[test]
    fn test_run_test_case_18() {
        // 使用通用测试函数测试用例18
        match run_test_case(18) {
            Ok(_) => println!("测试用例18通过！"),
            Err(e) => panic!("测试用例18失败: {}", e),
        }
    }

    #[test]
    fn test_all_public_cases() {
        // 测试指定的公开测试用例
        let test_cases = vec![1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 18];
        let mut passed_cases = Vec::new();
        let mut failed_cases = Vec::new();
        
        for case_num in test_cases {
            println!("运行测试用例 {}", case_num);
            match run_test_case(case_num) {
                Ok(_) => {
                    println!("✅ 测试用例 {} 通过！", case_num);
                    passed_cases.push(case_num);
                }
                Err(e) => {
                    println!("❌ 测试用例 {} 失败: {}", case_num, e);
                    failed_cases.push((case_num, e));
                }
            }
            println!(""); // 添加空行分隔
        }
        
        // 总结报告
        println!("=== 测试结果总结 ===");
        println!("通过的测试用例: {:?}", passed_cases);
        if !failed_cases.is_empty() {
            println!("失败的测试用例:");
            for (case_num, error) in &failed_cases {
                println!("  - 用例 {}: {}", case_num, error);
            }
            panic!("有 {} 个测试用例失败", failed_cases.len());
        } else {
            println!("🎉 所有 {} 个公开测试用例都通过了！", passed_cases.len());
        }
    }
}