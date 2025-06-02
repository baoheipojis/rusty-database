// lib.rs for executor crate
pub mod error;
pub mod formatter;
pub mod handler;
pub mod utils;

use storage::storage_engine_interface::StorageEngine;

/// 执行 SQL 字符串并返回输出结果
/// 
/// # 参数
/// * `sql` - SQL 字符串
/// * `storage_engine` - 存储引擎实现
/// 
/// # 返回值
/// * `Result<String, String>` - 成功时返回查询输出结果，失败时返回错误信息
pub fn execute_sql<T: StorageEngine>(
    sql: &str,
    storage_engine: &mut T,
) -> Result<String, String> {
    // 直接调用 handler.rs 中的字符串处理函数
    handler::execute_sql_and_get_output(sql, storage_engine)
}

