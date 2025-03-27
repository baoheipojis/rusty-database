use crate::storage::StorageEngine; // 引入存储引擎，crate表示根模块
use sqlparser::ast::{Query, Statement}; // 引入 SQL AST 类型
use crate::executor::error::ExecutionError; // 引入错误定义

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

/// 格式化并输出结果
fn format_and_print_results(rows: Vec<Row>) {
    println!("Results:");
    for row in rows {
        println!("{:?}", row);
    }
}
