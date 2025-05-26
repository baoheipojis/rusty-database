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

## CI/CD

项目使用 GitHub Actions 进行持续集成，每次推送代码时会自动：

- 🔧 检查代码格式化
- 🕵️ 运行 Clippy 代码检查
- 🏗️ 构建项目
- 🧪 运行所有测试
- 🌍 在多个平台上测试（Ubuntu, Windows, macOS）

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

## 贡献指南

1. Fork 此项目
2. 创建你的特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交你的修改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 打开一个 Pull Request

## 许可证

此项目使用 MIT 许可证 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 作者

- **[Your Name]** - *初始工作* - [Your GitHub](https://github.com/[YOUR_GITHUB_USERNAME])

## 致谢

- 感谢 [sqlparser-rs](https://github.com/sqlparser-rs/sqlparser-rs) 提供的 SQL 解析功能
- 感谢 Rust 社区的优秀生态系统