// bplus_tree.rs

#[derive(Debug)]
struct BPlusTreeNode {
    is_leaf: bool,                     // 节点是否为叶子节点
    keys: Vec<i32>,                   // 节点中的关键字
    children: Vec<Box<BPlusTreeNode>>, // 孩子节点的引用
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
            root: Box::new(BPlusTreeNode {
                is_leaf: true,       // 初始时根节点为叶子节点
                keys: Vec::new(),    // 关键字初始为空
                children: Vec::new(), // 孩子节点初始为空
            }),
            degree,
        }
    }

    /// 插入关键字
    /// 参数: key - 要插入的关键字
    pub fn insert(&mut self, key: i32) {
        let root = &mut *self.root;

        // 如果根节点已满，则树高度增加
        if root.keys.len() == (2 * self.degree) - 1 {
            // 创建新的根节点
            let new_root = Box::new(BPlusTreeNode {
                is_leaf: false,
                keys: Vec::new(),
                children: vec![self.root.clone()],
            });
            self.root = new_root; // 更新根节点
            self.split_child(&mut self.root, 0); // 分裂根节点
            self.insert_non_full(&mut self.root, key); // 在新根节点中插入
        } else {
            self.insert_non_full(root, key); // 在非满节点中插入
        }
    }

    /// 在非满节点中插入关键字
    /// 参数: node - 当前节点, key - 要插入的关键字
    fn insert_non_full(&mut self, node: &mut BPlusTreeNode, key: i32) {
        if node.is_leaf {
            // 在叶节点中插入关键字
            let pos = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());
            node.keys.insert(pos, key); // 按顺序插入关键字
        } else {
            // 在非叶子节点中查找孩子节点
            let pos = node.keys.iter().position(|&k| k > key).unwrap_or(node.keys.len());
            self.insert_non_full(&mut node.children[pos], key); // 递归插入
        }
    }

    /// 分裂子节点，并在父节点中插入提升的关键字
    /// 参数: parent - 父节点, index - 子节点的索引
    fn split_child(&mut self, parent: &mut BPlusTreeNode, index: usize) {
        let child = &mut parent.children[index]; // 获取要分裂的子节点
        let new_node = Box::new(BPlusTreeNode {
            is_leaf: child.is_leaf, // 新节点是否为叶子节点
            keys: child.keys.split_off(self.degree - 1), // 分裂关键字
            children: if child.is_leaf { vec![] } else { child.children.split_off(self.degree) }, // 分裂孩子节点
        });

        parent.keys.insert(index, child.keys.pop().unwrap()); // 提升关键字到父节点
        parent.children.insert(index + 1, new_node); // 添加新创建的子节点
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