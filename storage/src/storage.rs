// storage/storage.rs

use std::collections::HashMap; // HashMap 用于存储表
use std::fs::{OpenOptions, File}; // 文件读写用
use std::io::{Read, Write}; // 输入输出操作
use serde::{Deserialize, Serialize}; // 序列化库
use crate::storage_engine_interface::*; // 引入接口定义

/// 数据库表结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Table {
    pub schema: Schema,   // 表结构
    pub rows: Vec<Row>,   // 存储行数据
}

/// 简单存储引擎
#[derive(Debug)]
pub struct SimpleStorageEngine {
    tables: HashMap<String, Table>, // 表名及其对应的数据结构
    file_path: String,               // 数据持久化文件路径
}

impl SimpleStorageEngine {
    /// 创建新的存储引擎实例
    pub fn new(file_path: &str) -> Self {
        let mut engine = Self {
            tables: HashMap::new(),
            file_path: file_path.to_string(),
        };
        engine.load_from_disk(); // 从文件加载数据
        engine
    }

    /// 从磁盘加载表数据
    /*fn load_from_disk(&mut self) {
        let mut file = File::open(&self.file_path).unwrap_or_else(|_| File::create(&self.file_path).unwrap());
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read data from disk");
        self.tables = serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new()); // 反序列化
    }*/

    // modified
    fn load_from_disk(&mut self) {
        let file_result = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.file_path);
    
        let mut file = file_result.expect("Failed to open or create data file");
    
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read data from disk");
    
        self.tables = serde_json::from_str(&contents).unwrap_or_else(|_| HashMap::new());
    }

    /// 将数据保存到磁盘
    fn save_to_disk(&self) {
        let serialized = serde_json::to_string(&self.tables).expect("Failed to serialize tables");
        let mut file = OpenOptions::new().write(true).create(true).open(&self.file_path).expect("Failed to open file");
        file.set_len(0).expect("Failed to clear file"); // 清空文件
        file.write_all(serialized.as_bytes()).expect("Failed to write to file"); // 写入
    }
}

// 实现 StorageEngine trait
impl StorageEngine for SimpleStorageEngine {
    fn create_table(&mut self, table_name: &str, schema: Schema) -> Result<(), String> {
        // 检查表是否已存在
        if self.tables.contains_key(table_name) {
            return Err(format!("Error: Table '{}' already exists.", table_name));
        }

        // 创建新表
        let table = Table {
            schema,
            rows: Vec::new(),
        };
        self.tables.insert(table_name.to_string(), table); // 添加表到 HashMap
        self.save_to_disk(); // 保存数据到磁盘
        Ok(())
    }

    fn drop_table(&mut self, table_name: &str) -> Result<(), String> {
        // 从 HashMap 中移除指定表
        if self.tables.remove(table_name).is_none() {
            return Err(format!("Error: Table '{}' does not exist.", table_name));
        }
        self.save_to_disk(); // 更新磁盘数据
        Ok(())
    }

    fn insert_row(&mut self, table_name: &str, row: Row) -> Result<(), String> {
        // 检查表是否存在
        let table = self.tables.get_mut(table_name).ok_or_else(|| format!("Error: Table '{}' does not exist.", table_name))?;
        
        // 验证行数据与 schema 匹配
        if row.values.len() != table.schema.columns.len() {
            return Err("Error: Row values do not match table schema.".to_string());
        }

        // 类型检查
        for (value, column) in row.values.iter().zip(&table.schema.columns) {
            if !value.matches_type(&column.data_type) {
                return Err(format!("Error: Value '{:?}' does not match column type '{}'.", value, column.data_type));
            }
        }
        // 插入新行
        table.rows.push(row);
        self.save_to_disk(); // 保存到磁盘
        Ok(())
    }

    fn update_rows(
        &mut self,
        table_name: &str,
        updates: Vec<(String, Value)>,
        condition: Option<Condition>
    ) -> Result<u64, String> {
        // 查询表
        let table = self.tables.get_mut(table_name).ok_or_else(|| format!("Error: Table '{}' does not exist.", table_name))?;

        let mut updated_count = 0;

        for row in table.rows.iter_mut() {
            // 检查条件是否满足
            if let Some(ref cond) = condition {
                match cond {
                    Condition::Equals(column_name, value) => {
                        let col_index = table.schema.columns.iter().position(|c| c.name == *column_name);
                        if col_index.is_none() || row.values[col_index.unwrap()] != *value {
                            continue; // 跳过不满足条件的行
                        }
                    }
                    _ => {}
                }
            }

            // 更新行
            for (col_name, new_value) in &updates {
                if let Some(col_index) = table.schema.columns.iter().position(|c| c.name == *col_name) {
                    row.values[col_index] = new_value.clone(); // 更新值
                }
            }
            updated_count += 1; // 更新计数
        }

        self.save_to_disk(); // 更新后保存
        Ok(updated_count)
    }

    fn delete_rows(&mut self, table_name: &str, condition: Option<Condition>) -> Result<u64, String> {
        // 查询表
        let table = self.tables.get_mut(table_name).ok_or_else(|| format!("Error: Table '{}' does not exist.", table_name))?;
        
        // 记录原始行数
        let original_length = table.rows.len();
        table.rows.retain(|row| {
            if let Some(ref cond) = condition {
                match cond {
                    Condition::Equals(column_name, value) => {
                        let col_index = table.schema.columns.iter().position(|c| c.name == *column_name);
                        if col_index.is_none() || row.values[col_index.unwrap()] != *value {
                            return true; // 保留这一行
                        }
                    }
                    _ => {}
                }
            }
            false // 删除这一行
        });

        // 返回删除计数
        let deleted_count = original_length - table.rows.len();
        self.save_to_disk(); // 保存数据
        Ok(deleted_count as u64)
    }

    fn select_rows(
        &self,
        table_name: &str,
        columns: Vec<String>,
        condition: Option<Condition>
    ) -> Result<Vec<Row>, String> {
        // 查询表
        let table = self.tables.get(table_name).ok_or_else(|| format!("Error: Table '{}' does not exist.", table_name))?;
        
        let mut results = Vec::new();
        
        // 遍历行
        for row in &table.rows {
            if let Some(ref cond) = condition {
                match cond {
                    Condition::Equals(column_name, value) => {
                        let col_index = table.schema.columns.iter().position(|c| c.name == *column_name);
                        if col_index.is_none() || row.values[col_index.unwrap()] != *value {
                            continue; // 跳过不满足条件的行
                        }
                    }
                    _ => {}
                }
            }

            // 根据列名选择数据
            let selected_values = if columns.contains(&"*".to_string()) {
                row.values.clone() // 查询所有列
            } else {
                let mut selected = Vec::new();
                for col_name in &columns {
                    if let Some(col_index) = table.schema.columns.iter().position(|c| c.name == *col_name) {
                        selected.push(row.values[col_index].clone()); // 查询指定列
                    }
                }
                selected // 返回选择的列
            };

            results.push(Row { values: selected_values }); // 添加到结果集中
        }

        Ok(results) // 返回查询结果
    }

    fn get_table_schema(&self, table_name: &str) -> Result<Schema, String> {
        // 获取表的 schema
        let table = self.tables.get(table_name).ok_or_else(|| format!("Error: Table '{}' does not exist.", table_name))?;
        Ok(table.schema.clone()) // 返回表结构
    }
}
