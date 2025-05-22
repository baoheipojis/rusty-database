// storage_engine_interface.rs

/// 数据库存储引擎接口
/// 定义了存储引擎的核心功能，用于数据的存储和查询。
pub trait StorageEngine {
    /// 创建一张表
    /// 参数:
    /// - `table_name`: 表的名称
    /// - `schema`: 表的结构定义，包括列名、数据类型和约束条件。
    /// 返回:
    /// - 成功时返回 `Ok(())`，失败时返回 `Err(错误信息)`。
    fn create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String>;

    /// 删除一张表
    /// 参数:
    /// - `table_name`: 要删除的表的名称。
    /// 返回:
    /// - 成功时返回 `Ok(())`，失败时返回 `Err(错误信息)`。
    fn drop_table(&mut self, table_name: &str) -> Result<(), String>;

    /// 插入一条记录
    /// 参数:
    /// - `table_name`: 表的名称。
    /// - `row`: 包含需要插入的行数据。
    /// 返回:
    /// - 成功时返回 `Ok(())`，失败时返回 `Err(错误信息)`。
    fn insert_row(&mut self, table_name: &str, row: Row) -> Result<(), String>;

    /// 更新满足条件的记录
    /// 参数:
    /// - `table_name`: 表的名称。
    /// - `updates`: 更新的字段名及新值组成的键值对列表。
    /// - `condition`: 可选的条件，用于筛选需要更新的记录。
    /// 返回:
    /// - 成功时返回受影响的行数 `Ok(u64)`，失败时返回 `Err(错误信息)`。
    fn update_rows(
        &mut self,
        table_name: &str,
        updates: Vec<(String, Value)>,
        condition: Option<Condition>
    ) -> Result<u64, String>;

    /// 删除满足条件的记录
    /// 参数:
    /// - `table_name`: 表的名称。
    /// - `condition`: 可选的条件，用于筛选需要删除的记录。
    /// 返回:
    /// - 成功时返回删除的行数 `Ok(u64)`，失败时返回 `Err(错误信息)`。
    fn delete_rows(&mut self, table_name: &str, condition: Option<Condition>) -> Result<u64, String>;

    /// 查询数据
    /// 参数:
    /// - `table_name`: 表的名称。
    /// - `columns`: 查询的列名列表。如果查询所有列，可传入 `vec!["*"]`。
    /// - `condition`: 可选的条件，用于筛选需要查询的记录。
    /// 返回:
    /// - 成功时返回包含行数据的 `Ok(Vec<Row>)`，失败时返回 `Err(错误信息)`。
    fn select_rows(
        &self,
        table_name: &str,
        columns: Vec<String>,
        condition: Option<Condition>
    ) -> Result<Vec<Row>, String>;

    /// 获取表的 Schema 定义
    /// 参数:
    /// - `table_name`: 表的名称。
    /// 返回:
    /// - 成功时返回表的 Schema 定义 `Ok(Schema)`，失败时返回 `Err(错误信息)`。
    fn get_table_schema(&self, table_name: &str) -> Result<Schema, String>;
}

/// 表的 Schema 定义
/// 描述表的结构，包括列名、数据类型和约束条件。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Schema {
    pub columns: Vec<ColumnDefinition>, // 列的定义列表
}

/// 表中的列定义
/// 每一列的名称、数据类型以及约束条件。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ColumnDefinition {
    pub name: String,              // 列的名称
    pub data_type: DataType,       // 列的数据类型
    pub constraints: Vec<Constraint>, // 列的约束条件（如主键、非空）
}

/// 表中的一行数据
/// 包含多个值，每个值对应一列的数据。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub values: Vec<Value>, // 值列表
}

/// 值类型
/// 表示表中每列可以存储的值。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Value {
    Int(i32),         // 整数值
    Varchar(String),  // 字符串值
    Null,             // 空值
}

/// 数据类型
/// 支持 INT 和 VARCHAR，并可以指定长度。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DataType {
    Int(u32),      // 整数类型及位数
    Varchar(u32),  // 字符串类型及最大长度
}

/// 列约束条件
/// 定义列的限制，如是否为主键、是否允许为空。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Constraint {
    PrimaryKey, // 该列为主键
    NotNull,    // 该列不能为空
}

/// 条件表达式
/// 用于表示 WHERE 子句中的条件。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Condition {
    Equals(String, Value),      // 列名等于某个值
    GreaterThan(String, Value), // 列名大于某个值
    LessThan(String, Value),    // 列名小于某个值
    IsNull(String),             // 列名的值为空
    IsNotNull(String),          // 列名的值不为空
}