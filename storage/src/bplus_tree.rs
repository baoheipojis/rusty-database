// bplus_tree.rs

#[derive(Debug, Clone)]
struct BPlusTreeNode {
    is_leaf: bool,                     // 节点是否为叶子节点
    keys: Vec<i32>,                   // 节点中的关键字
    children: Vec<Box<BPlusTreeNode>>, // 孩子节点的引用
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
            root: Box::new(BPlusTreeNode::new_leaf()), // Root starts as a leaf
            degree,
        }
    }

    /// 插入关键字
    /// 参数: key - 要插入的关键字
    pub fn insert(&mut self, key: i32) {
        let degree = self.degree;
        
        if self.root.keys.len() == (2 * degree) - 1 { // Root is full
            // old_root_box now owns the previous root tree.
            let old_root_box = std::mem::replace(&mut self.root, Box::new(BPlusTreeNode::new_internal())); 
            
            // self.root is now a new, empty internal node.
            // Make old_root_box its first child.
            self.root.children.push(old_root_box);
            
            // Now, self.root is the parent, and its child at index 0 is the (full) old root.
            // Split this child.
            Self::split_child_node(degree, &mut *self.root, 0);
            
            // After split, self.root has one key and two children.
            // Insert the key into this (no longer empty, not full) new root structure.
            Self::insert_non_full_node(degree, &mut *self.root, key);
        } else { // Root is not full
            Self::insert_non_full_node(degree, &mut *self.root, key);
        }
    }

    /// 在非满节点中插入关键字 (static helper)
    /// 参数: degree - 树的阶, node - 当前节点, key - 要插入的关键字
    fn insert_non_full_node(degree: usize, node: &mut BPlusTreeNode, key: i32) {
        if node.is_leaf {
            let pos = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());
            node.keys.insert(pos, key);
        } else { // node is internal
            let mut insertion_point_idx = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());
            
            // Check if the child we are about to descend into is full
            if node.children[insertion_point_idx].keys.len() == (2 * degree) - 1 {
                Self::split_child_node(degree, node, insertion_point_idx);
                
                // After split, the key `key` might go to the child at `insertion_point_idx` or `insertion_point_idx + 1`.
                // `node.keys[insertion_point_idx]` is the key that was promoted from the split child.
                if key > node.keys[insertion_point_idx] {
                    insertion_point_idx += 1; 
                }
            }
            // Now, child at node.children[insertion_point_idx] is guaranteed not full.
            Self::insert_non_full_node(degree, &mut node.children[insertion_point_idx], key);
        }
    }

    /// 分裂子节点 (static helper)
    /// 参数: degree - 树的阶, parent_node - 父节点, child_index - 子节点的索引
    fn split_child_node(degree: usize, parent_node: &mut BPlusTreeNode, child_index: usize) {
        let mut new_sibling_node_content = BPlusTreeNode {
            is_leaf: parent_node.children[child_index].is_leaf,
            keys: Vec::new(),
            children: Vec::new(),
        };

        let median_key;

        // Scoped borrow for child_to_split_node
        {
            let child_to_split_node = &mut parent_node.children[child_index]; // &mut Box<BPlusTreeNode>
            
            // Median key is at index `degree - 1`
            median_key = child_to_split_node.keys[degree - 1];

            // Move keys from `degree` onwards to new_sibling_node_content
            new_sibling_node_content.keys = child_to_split_node.keys.drain(degree..).collect();
            
            // Remove the median key from child_to_split_node's keys (it's already copied)
            // and keep keys 0 to degree-2
            child_to_split_node.keys.truncate(degree - 1);

            if !child_to_split_node.is_leaf {
                // Move children from `degree` onwards to new_sibling_node_content
                new_sibling_node_content.children = child_to_split_node.children.drain(degree..).collect();
                // Keep children 0 to degree-1
                child_to_split_node.children.truncate(degree);
            }
        } // child_to_split_node borrow ends

        parent_node.keys.insert(child_index, median_key);
        parent_node.children.insert(child_index + 1, Box::new(new_sibling_node_content));
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
        // 递归查找子节点
        self._search(&node.children[pos], key)
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
