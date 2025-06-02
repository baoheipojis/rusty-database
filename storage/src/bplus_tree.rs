#[derive(Debug, Clone)]
struct BPlusTreeNode {
    is_leaf: bool,                          // 节点是否为叶子节点
    keys: Vec<i32>,                        // 节点中的关键字，使用 i32 类型
    children: Vec<Box<BPlusTreeNode>>,      // 子节点的引用
}

impl BPlusTreeNode {
    fn new_leaf() -> Self {
        BPlusTreeNode {
            is_leaf: true,
            keys: Vec::new(),
            children: Vec::new(),
        }
    }

    fn new_internal() -> Self {
        BPlusTreeNode {
            is_leaf: false,
            keys: Vec::new(),
            children: Vec::new(),
        }
    }
}

pub struct BPlusTree {
    root: Box<BPlusTreeNode>, // B+树的根节点
    degree: usize,             // B+树的最小度数
}

impl BPlusTree {
    /// 创建新的 B+树
    /// 参数: degree - 树的阶（最小度数）
    pub fn new(degree: usize) -> Self {
        Self {
            root: Box::new(BPlusTreeNode::new_leaf()), // 根节点初始化为叶子节点
            degree,
        }
    }

    /// 插入关键字
    /// 参数: key - 要插入的关键字
    pub fn insert(&mut self, key: i32) -> Result<(), String> {
        if key < 0 {
            return Err("插入的关键字不能为负数".to_string()); // 非空约束
        }

        // 根节点已满时，进行分裂
        if self.root.keys.len() == (2 * self.degree) - 1 {
            let old_root_box = std::mem::replace(&mut self.root, Box::new(BPlusTreeNode::new_internal()));
            self.root.children.push(old_root_box);
            Self::split_child_node(self.degree, &mut *self.root, 0)?;
        }

        // 在非满节点中插入关键字
        Self::insert_non_full_node(self.degree, &mut *self.root, key)
    }

    /// 在非满节点中插入关键字 (static helper)
    /// 参数: degree - 树的阶, node - 当前节点, key - 要插入的关键字
    fn insert_non_full_node(degree: usize, node: &mut BPlusTreeNode, key: i32) -> Result<(), String> {
        if node.is_leaf {
            // 检查主键唯一性
            if node.keys.contains(&key) {
                return Err(format!("主键 '{}' 已存在", key)); // 唯一性约束
            }
            let pos = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());
            node.keys.insert(pos, key); // 在节点中找到合适位置插入
            Ok(())
        } else { // 节点为内部节点
            let mut insertion_point_idx = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());

            // 检查待插入子节点是否已满
            if node.children[insertion_point_idx].keys.len() == (2 * degree) - 1 {
                Self::split_child_node(degree, node, insertion_point_idx)?;

                // 若关键字大于当前节点的中间值，则选择下一个子节点
                if key > node.keys[insertion_point_idx] {
                    insertion_point_idx += 1;
                }
            }
            // 在保证子节点不满的前提下，继续插入
            Self::insert_non_full_node(degree, &mut node.children[insertion_point_idx], key)
        }
    }

    /// 分裂子节点 (static helper)
    /// 参数: degree - 树的阶, parent_node - 父节点, child_index - 子节点的索引
    fn split_child_node(degree: usize, parent_node: &mut BPlusTreeNode, child_index: usize) -> Result<(), String> {
        let mut new_sibling_node_content = BPlusTreeNode {
            is_leaf: parent_node.children[child_index].is_leaf,
            keys: Vec::new(),
            children: Vec::new(),
        };

        let median_key;

        {
            let child_to_split_node = &mut parent_node.children[child_index]; // 获取需要分裂的节点
            
            median_key = child_to_split_node.keys[degree - 1]; // 中间键
            
            // 移动关键字
            new_sibling_node_content.keys = child_to_split_node.keys.split_off(degree);
            // 截断原节点的关键字
            child_to_split_node.keys.truncate(degree - 1);

            if !child_to_split_node.is_leaf {
                // 移动子节点
                new_sibling_node_content.children = child_to_split_node.children.split_off(degree);
                child_to_split_node.children.truncate(degree);
            }
        }

        // 将中间键插入到父节点中
        parent_node.keys.insert(child_index, median_key);
        parent_node.children.insert(child_index + 1, Box::new(new_sibling_node_content));
        Ok(())
    }

    /// 在树中查找关键字
    /// 参数: key - 要查找的关键字
    /// 返回: 如果关键字存在，返回 true，否则返回 false
    pub fn search(&self, key: i32) -> bool {
        self._search(&self.root, key)
    }

    /// 在节点内部查找关键字
    /// 参数: node - 当前节点, key - 要查找的关键字
    /// 返回: 如果关键字存在，返回 true，否则返回 false
    fn _search(&self, node: &BPlusTreeNode, key: i32) -> bool {
        let pos = node.keys.iter().position(|&k| k >= key).unwrap_or(node.keys.len()); // 查找位置
        if pos > 0 && node.keys[pos - 1] == key {
            return true; // 找到关键字
        }
        if node.is_leaf {
            return false; // 到达叶子节点，仍未找到
        }
        self._search(&node.children[pos], key) // 递归查找子节点
    }

    /// 按顺序查找并返回关键字集合
    pub fn search_sorted(&self) -> Vec<i32> {
        let mut result = Vec::new();
        self.collect_keys(&self.root, &mut result);
        result.sort_unstable(); // 无需稳定排序
        result
    }

    fn collect_keys(&self, node: &BPlusTreeNode, result: &mut Vec<i32>) {
        if node.is_leaf {
            result.extend_from_slice(&node.keys); // 收集叶子节点的所有关键字
        } else {
            for (i, key) in node.keys.iter().enumerate() {
                self.collect_keys(&node.children[i], result); // 递归收集子节点
                result.push(*key); // 插入当前节点的关键字
            }
            // 访问最后一个子节点
            self.collect_keys(&node.children[node.children.len() - 1], result);
        }
    }

    /// 打印 B+树结构
    /// 用于调试和可视化树结构
    pub fn print_tree(&self) {
        self._print_tree(&self.root, 0);
    }

    /// 递归打印节点
    /// 参数: node - 当前节点, level - 当前层级
    fn _print_tree(&self, node: &BPlusTreeNode, level: usize) {
        println!("Level {}: {:?}", level, node.keys); // 打印当前层级的关键字
        for child in &node.children {
            self._print_tree(child, level + 1); // 递归打印子节点
        }
    }
}

// 单元测试模块
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let mut tree = BPlusTree::new(3); // 创建一个最小度数为 3 的 B+ 树

        // 测试插入
        assert!(tree.insert(10).is_ok());
        assert!(tree.insert(20).is_ok());
        assert!(tree.insert(5).is_ok());
        assert!(tree.insert(15).is_ok());
        
        // 测试搜索
        assert!(tree.search(10));
        assert!(tree.search(20));
        assert!(!tree.search(100)); // 100 应该不存在

        // 测试插入重复主键
        assert!(tree.insert(10).is_err()); // 10 已存在
    }

    #[test]
    fn test_search_sorted() {
        let mut tree = BPlusTree::new(3);
        
        // 插入一些数据
        tree.insert(10).unwrap();
        tree.insert(30).unwrap();
        tree.insert(20).unwrap();
        
        // 检查排序结果
        let sorted_keys = tree.search_sorted();
        assert_eq!(sorted_keys, vec![10, 20, 30]);
    }

    #[test]
    fn test_insert_negative_key() {
        let mut tree = BPlusTree::new(3);
        assert!(tree.insert(-1).is_err()); // 负数插入应该失败
    }
}
