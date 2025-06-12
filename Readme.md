# Rusty Database

![CI](https://github.com/[YOUR_GITHUB_USERNAME]/rusty-database/workflows/CI/badge.svg)

一个用 Rust 编写的简单数据库实现，支持基本的 SQL 操作。

## 特性

- 🚀 **SQL 解析**: 支持基本的 SQL 语句解析（CREATE TABLE, INSERT, SELECT）
- 💾 **存储引擎**: 模块化的存储引擎接口
- 🔍 **查询执行**: 支持带条件的查询操作
- 🧪 **全面测试**: 包含完整的单元测试和集成测试

## 项目结构

```
rusty-database/
├── executor/          # SQL 执行引擎
│   ├── src/
│   │   ├── handler.rs     # SQL 语句处理器
│   │   ├── error.rs       # 错误定义
│   │   ├── formatter.rs   # 结果格式化
│   │   └── lib.rs         # 模块入口
├── parser/            # SQL 解析器
├── storage/           # 存储引擎
│   ├── src/
│   │   ├── storage_engine_interface.rs  # 存储引擎接口
│   │   ├── storage.rs                   # 存储实现
│   │   └── bplus_tree.rs               # B+ 树索引
└── src/               # 主程序
    ├── main.rs        # 程序入口
    └── lib.rs         # 库入口
```

## 快速开始

### 环境要求

- Rust 1.70+ 
- Cargo

### 安装和运行

1. 克隆项目：
```bash
git clone https://github.com/[YOUR_GITHUB_USERNAME]/rusty-database.git
cd rusty-database
```

2. 构建项目：
```bash
cargo build
```

3. 运行测试：
```bash
cargo test
```

4. 运行主程序：
```bash
cargo run
```

## 支持的 SQL 语句
我们项目支持CREATE, DROP, INSERT, SELECT等基本的SQL语句。
### CREATE TABLE
```sql
CREATE TABLE users (
    id INT,
    name VARCHAR(100),
    age INT
);
```

### INSERT
```sql
INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);
INSERT INTO users (id, name, age) VALUES (2, 'Bob', 25), (3, 'Charlie', 35);
```

### SELECT
```sql
SELECT * FROM users;
SELECT id, name FROM users WHERE age > 25;
SELECT id, name FROM users WHERE name = 'Alice';
```

## 开发

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定模块的测试
cargo test -p executor
cargo test -p storage

# 详细输出
cargo test -- --nocapture
```

### 代码格式化
```bash
cargo fmt
```

### 代码检查
```bash
cargo clippy
```

## 架构设计

### 模块化设计

- **Parser**: 负责将 SQL 字符串解析为 AST
- **Executor**: 负责执行 SQL AST，协调各个组件
- **Storage**: 提供数据存储和检索功能

### 存储引擎接口

```rust
pub trait StorageEngine {
    fn create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String>;
    fn insert_row(&mut self, table_name: &str, row: Row) -> Result<(), String>;
    fn select_rows(&self, table_name: &str, columns: Vec<String>, condition: Option<Condition>) -> Result<Vec<Row>, String>;
    // ... 更多方法
}
```
### 执行引擎

我们的执行引擎是数据库系统的核心组件，负责解析和执行 SQL 语句。执行引擎采用模块化设计，通过统一的接口与存储引擎交互。

#### 核心功能

- **SQL 语句解析**: 使用 `sqlparser-rs` 库将 SQL 字符串解析为抽象语法树（AST）
- **语句执行**: 支持 CREATE TABLE、INSERT、SELECT、UPDATE、DELETE 等基本 SQL 操作
- **查询处理**: 支持带条件的查询、表达式计算和复合条件查询
- **错误处理**: 提供完整的错误信息和异常处理机制
- **结果格式化**: 将查询结果格式化为表格形式输出

#### 执行引擎接口

```rust
/// 执行 SQL 字符串并返回输出结果
pub fn execute_sql_and_get_output<T: StorageEngine>(
    input_sql: &str,
    storage_engine: &mut T
) -> Result<String, String>;

/// 执行单个 SQL 语句
pub fn execute_stmt(
    statement: Statement,
    storage_engine: &mut dyn StorageEngine,
) -> Result<QueryResult, ExecutionError>;
```

#### 支持的语句类型

| 语句类型 | 处理函数 | 说明 |
|---------|---------|------|
| `SELECT` | `handle_query_stmt` | 查询数据，支持条件过滤和表达式计算 |
| `INSERT` | `handle_insert_stmt` | 插入数据，支持单行和多行插入 |
| `CREATE TABLE` | `handle_create_table_stmt` | 创建表，支持约束定义 |
| `DROP TABLE` | `handle_drop_table_stmt` | 删除表 |


#### 查询结果类型

```rust
pub enum QueryResult {
    Data(Vec<Row>),         // SELECT 查询返回的数据行
    RowsAffected(u64),      // UPDATE/DELETE 影响的行数
    Success,                // CREATE/DROP 等操作的成功标识
}
```

#### 错误处理

执行引擎定义了完整的错误类型体系：

```rust
pub enum ExecutionError {
    StorageError(String),     // 存储引擎错误
    SyntaxError,              // SQL 语法错误  
    UnsupportedStatement,     // 不支持的语句类型
    TableNotFound(String),    // 表不存在
    ColumnNotFound(String),   // 列不存在
}
```

## 贡献指南

1. Fork 此项目
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交你的修改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开一个 Pull Request

## 许可证

此项目使用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 致谢

- 感谢 [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs) 提供的 SQL 解析功能
- 感谢 Rust 社区的优秀生态系统