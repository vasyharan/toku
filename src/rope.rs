use std::rc::Rc;

#[derive(Debug)]
enum Error {
    EOS,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Position {
    ByteOffset(usize),
    // LineAndColumn((u32, u32)),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NodeColour {
    Red,
    Black,
}

impl std::fmt::Display for NodeColour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeColour::Red => write!(f, "red")?,
            NodeColour::Black => write!(f, "black")?,
        }
        Ok(())
    }
}

enum Node {
    Branch {
        colour: NodeColour,
        left: Rc<Node>,
        right: Rc<Node>,
        len: usize,
    },
    Leaf {
        val: String,
        len: usize,
    },
}

use Node::{Branch, Leaf};

impl Node {
    fn new_branch(colour: NodeColour, left: Rc<Node>, right: Rc<Node>) -> Self {
        let len = left.len() + right.len();
        Branch {
            colour,
            left,
            right,
            len,
        }
    }
    fn new_leaf(val: String) -> Self {
        let len = val.len();
        Leaf { val, len }
    }

    fn len(&self) -> usize {
        match &self {
            Branch { len, .. } => *len,
            Leaf { len, .. } => *len,
        }
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            Branch {
                colour,
                left,
                right,
                ..
            } => {
                write!(
                    w,
                    "\tn{:p}[shape=circle,color={},label=\"\"];\n",
                    self, colour
                )?;

                left.write_dot(w)?;
                write!(
                    w,
                    "\tn{:p} -> n{:p}[label=\"{}\"];\n",
                    self,
                    left.as_ref(),
                    left.len()
                )?;

                right.write_dot(w)?;
                write!(
                    w,
                    "\tn{:p} -> n{:p}[label=\"{}\"];\n",
                    self,
                    right.as_ref(),
                    right.len()
                )?;
            }
            Leaf { val, .. } => {
                write!(w, "\tn{:p}[shape=square,label=\"'{}'\"];\n", self, val)?;
            }
        }
        Ok(())
    }
}

impl ToString for Node {
    fn to_string(&self) -> String {
        match self {
            Leaf { val, .. } => val.clone(),
            Branch { left, right, .. } => {
                let mut s = left.to_string();
                s.push_str(&right.to_string());
                s
            }
        }
    }
}
struct Rope {
    root: Rc<Node>,
}

impl Rope {
    fn empty() -> Self {
        let root = Rc::new(Node::new_leaf("".to_string()));
        Self { root }
    }

    fn len(&self) -> usize {
        self.root.len()
    }

    fn insert_at(&self, pos: Position, text: String) -> Result<Self, Error> {
        if text.len() == 0 {
            return Ok(Self {
                root: self.root.clone(),
            });
        }
        match pos {
            Position::ByteOffset(offset) => {
                if offset > self.root.len() {
                    return Err(Error::EOS);
                }
                let root = insert(&self.root, offset, text);
                let root = make_black(root);
                Ok(Self {
                    root: Rc::new(root),
                })
            }
        }
    }

    fn delete_at(&self, pos: Position, len: usize) -> Result<Self, Error> {
        if len == 0 {
            return Ok(Self {
                root: self.root.clone(),
            });
        }
        match pos {
            Position::ByteOffset(offset) => {
                let (maybe_root, len) = delete(&self.root, offset, len);
                match maybe_root {
                    None => Ok(Self::empty()),
                    Some(root) => Ok(Self { root }),
                }
            }
        }
    }

    fn is_balanced(&self) -> bool {
        is_node_balanced(&self.root, 0).0
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        write!(w, "digraph {{\n")?;
        self.root.write_dot(w)?;
        write!(w, "}}")?;
        Ok(())
    }
}

impl ToString for Rope {
    fn to_string(&self) -> String {
        self.root.to_string()
    }
}

fn is_node_balanced(node: &Node, black_depth: usize) -> (bool, usize) {
    match node {
        Node::Leaf { .. } => (true, black_depth),
        Node::Branch {
            colour,
            left,
            right,
            ..
        } => {
            if *colour == NodeColour::Red {
                if let Branch {
                    colour: NodeColour::Red,
                    ..
                } = left.as_ref()
                {
                    return (false, 0);
                }
                if let Branch {
                    colour: NodeColour::Red,
                    ..
                } = right.as_ref()
                {
                    return (false, 0);
                }
            }

            let black_depth = black_depth
                + (match colour {
                    NodeColour::Red => 0,
                    NodeColour::Black => 1,
                });
            let lbal = is_node_balanced(left.as_ref(), black_depth);
            let rbal = is_node_balanced(right.as_ref(), black_depth);
            return if lbal == rbal { lbal } else { (false, 0) };
        }
    }
}

fn make_black(node: Node) -> Node {
    match node {
        Branch {
            colour: NodeColour::Red,
            left,
            right,
            len,
        } => Branch {
            colour: NodeColour::Black,
            left,
            right,
            len,
        },
        _ => node,
    }
}

fn insert(node: &Node, offset: usize, text: String) -> Node {
    match node {
        Leaf { val, .. } => {
            if node.len() == 0 {
                Node::new_leaf(text)
            } else {
                let left = Rc::new(Node::new_leaf(val[0..offset].to_string()));
                let right = Rc::new(Node::new_leaf(text));
                Node::new_branch(NodeColour::Red, left, right)
            }
        }
        Branch {
            colour,
            left,
            right,
            ..
        } => {
            let left_len = left.len();
            if left_len > offset {
                let left = insert(left.as_ref(), offset, text);
                balance(*colour, Rc::new(left), right.clone())
            } else {
                let offset = offset - left_len;
                let right = insert(right.as_ref(), offset, text);
                balance(*colour, left.clone(), Rc::new(right))
            }
        }
    }
}

fn delete(node: &Node, offset: usize, len: usize) -> (Option<Rc<Node>>, usize) {
    match node {
        Leaf { val, .. } => match (offset == 0, (offset + len) >= val.len()) {
            (false, false) => {
                let left = Node::new_leaf(val[..offset].to_string());
                let right = Node::new_leaf(val[(offset + len)..].to_string());
                let node = Node::new_branch(NodeColour::Red, Rc::new(left), Rc::new(right));
                (Some(Rc::new(node)), 0)
            }
            (false, true) => {
                let node = Node::new_leaf(val[..offset].to_string());
                (Some(Rc::new(node)), len - (val.len() - offset))
            }
            (true, false) => {
                let node = Node::new_leaf(val[(offset + len)..].to_string());
                (Some(Rc::new(node)), 0)
            }
            (true, true) => (None, len - val.len()),
        },
        Branch {
            colour,
            left,
            right,
            ..
        } => {
            let left_len = left.len();
            if left_len > offset {
                match delete(left.as_ref(), offset, len) {
                    (None, 0) => (Some(right.clone()), 0),
                    (None, len) => delete(right.as_ref(), 0, len),
                    (Some(left), 0) => {
                        let node = balance(*colour, left, right.clone());
                        (Some(Rc::new(node)), 0)
                    }
                    (Some(left), len) => match delete(right.as_ref(), 0, len) {
                        (None, len) => (Some(left.clone()), len),
                        (Some(right), len) => {
                            let node = balance(*colour, left, right);
                            (Some(Rc::new(node)), len)
                        }
                    },
                }
            } else {
                match delete(right.as_ref(), offset - left_len, len) {
                    (None, len) => (Some(left.clone()), len),
                    (Some(right), len) => {
                        let node = balance(*colour, left.clone(), right);
                        (Some(Rc::new(node)), len)
                    }
                }
            }
        }
    }
}

fn balance(colour: NodeColour, left: Rc<Node>, right: Rc<Node>) -> Node {
    use NodeColour::{Black, Red};
    if colour == Red {
        return Node::new_branch(colour, left, right);
    }

    match (left.as_ref(), right.as_ref()) {
        #[rustfmt::skip]
        (Branch { colour: Red, left: ll, right: lr, ..  }, _) => {
            if let Branch { colour: Red, left: a, right: b, ..  } = ll.as_ref() {
                let c = lr;
                let d = right;
                let l = Node::new_branch(Black, a.clone(), b.clone());
                let r = Node::new_branch(Black, c.clone(), d.clone());
                Node::new_branch(Red, Rc::new(l), Rc::new(r))
            } else if let Branch { colour: Red, left: b, right: c, ..  } = lr.as_ref() {
                let a = ll;
                let d = right;
                let l = Node::new_branch(Black, a.clone(), b.clone());
                let r = Node::new_branch(Black, c.clone(), d.clone());
                Node::new_branch(Red, Rc::new(l), Rc::new(r))
            } else {
                Node::new_branch(colour, left, right)
            }
        }
        #[rustfmt::skip]
        (_, Branch { colour: Red, left: rl, right: rr, ..  }) => {
            if let Branch { colour: Red, left: b, right: c, ..  } = rl.as_ref() {
                let a = left;
                let d = rr;
                let l = Node::new_branch(Black, a.clone(), b.clone());
                let r = Node::new_branch(Black, c.clone(), d.clone());
                Node::new_branch(Red, Rc::new(l), Rc::new(r))
            } else if let Branch { colour: Red, left: c, right: d, ..  } = rr.as_ref() {
                let a = left;
                let b = rl.clone();
                let l = Node::new_branch(Black, a.clone(), b.clone());
                let r = Node::new_branch(Black, c.clone(), d.clone());
                Node::new_branch(Red, Rc::new(l), Rc::new(r))
            } else {
                Node::new_branch(colour, left, right)
            }
        }
        (_, _) => Node::new_branch(colour, left, right),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tests() {
        std::fs::create_dir_all("tmp/").expect("create directory");
        let parts = vec![
            "Lorem ",       // 1
            "ipsum ",       // 2
            "dolor ",       // 3
            "sit ",         // 4
            "amet ",        // 5
            "consectetur ", // 6
            "adipiscing ",  // 7
            "elit ",        // 8
            "sed ",         // 9
            "do ",          // 10
            "eiusmod ",     // 11
            "tempor ",      // 12
            "incididunt ",  // 13
            "ut ",          // 14
            "labore ",      // 15
            "et ",          // 16
            "dolore ",      // 17
            "magna ",       // 18
            "aliqua",       // 19
            ".",            // 20
        ];

        let mut rope = Rope::empty();
        assert!(rope.is_balanced());

        for (i, &p) in parts.iter().enumerate() {
            rope = rope
                .insert_at(Position::ByteOffset(rope.len()), p.to_string())
                .unwrap();

            let mut file =
                std::fs::File::create(format!("tmp/insert{:02}.dot", i)).expect("create file");
            rope.write_dot(&mut file).expect("write dot file");
            assert!(rope.is_balanced());
        }
        assert_eq!(rope.to_string(), "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");

        rope = rope.delete_at(Position::ByteOffset(2), 2).unwrap();
        let mut file = std::fs::File::create("tmp/delete00.dot").expect("create file");
        rope.write_dot(&mut file).expect("write dot file");
        assert_eq!(rope.to_string(), "Lom ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        assert!(rope.is_balanced());

        rope = rope.delete_at(Position::ByteOffset(0), 1).unwrap();
        let mut file = std::fs::File::create("tmp/delete01.dot").expect("create file");
        rope.write_dot(&mut file).expect("write dot file");
        assert_eq!(rope.to_string(), "om ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        assert!(rope.is_balanced());

        rope = rope.delete_at(Position::ByteOffset(2), 1).unwrap();
        let mut file = std::fs::File::create("tmp/delete02.dot").expect("create file");
        rope.write_dot(&mut file).expect("write dot file");
        assert_eq!(rope.to_string(), "omipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        assert!(rope.is_balanced());

        rope = rope.delete_at(Position::ByteOffset(10), 22).unwrap();
        let mut file = std::fs::File::create("tmp/delete03.dot").expect("create file");
        rope.write_dot(&mut file).expect("write dot file");
        assert_eq!(rope.to_string(), "omipsum dour adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        assert!(rope.is_balanced());
    }
}
