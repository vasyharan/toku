mod tree;
use tree::Tree;

#[derive(Debug)]
pub enum Error {
    EOS,
}

#[derive(Debug, Clone)]
pub struct Rope {
    root: Tree,
}

impl Rope {
    pub fn empty() -> Self {
        Self::from_string("")
    }

    pub fn from_string(str: &str) -> Self {
        Self { root: Tree::from_str(str) }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn insert_at(&self, offset: usize, text: String) -> Result<Self, Error> {
        if text.len() == 0 {
            return Ok(Self { root: self.root.clone() });
        }
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        let root = self.root.insert_at(offset, text);
        Ok(Self { root })
    }

    pub fn delete_at(&self, offset: usize, len: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() || len + offset > self.root.len() {
            return Err(Error::EOS);
        }
        match self.root.delete_at(offset, len) {
            (left, right) => Ok((Self { root: (left) }, Self { root: (right) })),
        }
    }

    fn split(&self, offset: usize) -> Result<(Self, Self), Error> {
        if offset > self.root.len() {
            return Err(Error::EOS);
        }
        match self.root.split(offset) {
            (None, None) => Ok((Self::empty(), Self::empty())),
            (None, Some(right)) => Ok((Self::empty(), Self { root: (right) })),
            (Some(left), None) => Ok((Self { root: (left) }, Self::empty())),
            (Some(left), Some(right)) => Ok((Self { root: (left) }, Self { root: (right) })),
        }
    }

    pub fn is_balanced(&self) -> bool {
        match &self.root.black_height() {
            Ok(_) => true,
            _ => false,
        }
    }

    fn write_dot(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        self.root.write_dot(w)
    }
}

impl ToString for Rope {
    fn to_string(&self) -> String {
        self.root.to_string()
    }
}

impl From<&str> for Rope {
    fn from(str: &str) -> Self {
        Rope::from_string(str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_tests() {
        let _ = std::fs::remove_dir_all("target/tests/");
        std::fs::create_dir_all("target/tests/").expect("create directory");
        let parts = vec![
            (0, "Some "),
            (5, "people "),
            (0, "It "),
            (15, "not "),
            (3, "just "),
            (24, "knowing "),
            (8, "goes and"),
            (28, "started "),
            (13, "'round "),
            (23, " 'round "),
            (51, "singing "),
            (71, "what was;\n"),
            (75, " it"),
            (30, ", my"),
            (63, "it\n"),
            (35, "frends.\n"),
            (37, "i"),
            (100, " forever"),
            (0, "This "),
            (113, "because..."),
            (5, " the"),
            (5, "is"),
            (111, "and "),
            (115, "they"),
            (11, "ends.\n"),
            (11, " never "),
            (133, "continue "),
            (11, " that"),
            (146, " singing"),
            (12, "song "),
            (159, " t"),
            (160, "i"),
            (170, " jt "),
            (172, "us"),
            (186, "\n"),
        ];
        let contents = "This is the song that never ends.\n\
            It just goes 'round and 'round, my friends.\n\
            Some people started singing it\n\
            not knowing what it was;\n\
            and they continue singing it forever just because...\n\
        ";

        let mut rope = Rope::empty();
        assert!(rope.is_balanced());

        for (i, (at, p)) in parts.iter().enumerate() {
            rope = rope.insert_at(*at, p.to_string()).unwrap();

            let mut file = std::fs::File::create(format!("target/tests/insert{:02}.dot", i))
                .expect("create file");
            rope.write_dot(&mut file).expect("write dot file");
            assert!(
                rope.is_balanced(),
                "unbalanced when inserting {:?} at {}",
                p,
                at
            );
        }
        assert!(rope.is_balanced());
        assert_eq!(rope.to_string(), contents);

        for at in 0..rope.len() {
            let (split_left, split_right) = rope.split(at).expect("split rope");

            let mut file = std::fs::File::create(format!("target/tests/split_left{:02}.dot", at))
                .expect("create file");
            split_left.write_dot(&mut file).expect("write dot file");
            let mut file = std::fs::File::create(format!("target/tests/split_right{:02}.dot", at))
                .expect("create file");
            split_right.write_dot(&mut file).expect("write dot file");

            assert_eq!(split_left.to_string(), contents[..at]);
            assert_eq!(split_right.to_string(), contents[at..]);

            assert!(split_left.is_balanced());
            assert!(split_right.is_balanced());
        }

        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(0, 1).expect("delete rope");

            let mut file =
                std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
                    .expect("create file");
            updated.write_dot(&mut file).expect("write dot file");
            let mut file =
                std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
                    .expect("create file");
            deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_string(), contents[i..]);
            assert_eq!(deleted.to_string().as_bytes(), [contents.as_bytes()[i - 1]]);
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        (1..=rope.len()).fold(rope.clone(), |rope, i| {
            let (updated, deleted) = rope.delete_at(rope.len() - 1, 1).expect("delete rope");

            let mut file =
                std::fs::File::create(format!("target/tests/delete_updated{:02}.dot", i))
                    .expect("create file");
            updated.write_dot(&mut file).expect("write dot file");
            let mut file =
                std::fs::File::create(format!("target/tests/delete_deleted{:02}.dot", i))
                    .expect("create file");
            deleted.write_dot(&mut file).expect("write dot file");

            assert_eq!(updated.to_string(), contents[..(rope.len() - 1)]);
            assert_eq!(
                deleted.to_string(),
                String::from_utf8(vec![contents.as_bytes()[rope.len() - 1]]).expect("utf8 string")
            );
            assert!(updated.is_balanced());
            assert!(deleted.is_balanced());
            updated
        });

        // rope = rope.delete_at(Position::ByteOffset(2), 2).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete00.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "Lom ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(0), 1).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete01.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "om ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(2), 1).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete02.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "omipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());

        // rope = rope.delete_at(Position::ByteOffset(10), 22).unwrap();
        // let mut file = std::fs::File::create("target/tests/delete03.dot").expect("create file");
        // rope.write_dot(&mut file).expect("write dot file");
        // assert_eq!(rope.to_string(), "omipsum dour adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.");
        // assert!(rope.is_balanced());
    }
}
