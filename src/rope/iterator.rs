use std::ops::Range;

use super::slice::RopeLine;
use super::tree::{self, LeafOffset, Node, NodeMetrics};
use super::Rope;

pub(crate) struct ChunkAndRanges<'a> {
    range: Range<usize>,
    parents: Vec<&'a Node>,
    curr_pos: Option<LeafOffset<'a>>,
    trim_last_terminator: bool,
}

impl<'a> ChunkAndRanges<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        let mut parents = vec![];
        let (leaf, _) = tree::leaf_at_byte_offset(&mut parents, rope.0.as_ref(), range.start);
        Self { range, parents, curr_pos: leaf, trim_last_terminator: false }
    }

    pub(super) fn new_trim_last_terminator(rope: &'a Rope, range: Range<usize>) -> Self {
        let mut parents = vec![];
        let (leaf, _) = tree::leaf_at_byte_offset(&mut parents, rope.0.as_ref(), range.start);
        Self { range, parents, curr_pos: leaf, trim_last_terminator: true }
    }
}

impl<'a> Iterator for ChunkAndRanges<'a> {
    type Item = (&'a [u8], Range<usize>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.range.is_empty() || self.curr_pos.is_none() {
            return None;
        }

        let (leaf, leaf_start) = self.curr_pos.unwrap();
        if let Node::Leaf { block_ref, metrics, .. } = leaf {
            let bytes = &block_ref.as_bytes()[leaf_start..];
            let chunk = if bytes.len() < self.range.len() {
                Some(bytes)
            } else {
                let chunk = Some(&bytes[..(self.range.len())]);
                if self.trim_last_terminator {
                    trim_last_terminator(chunk)
                } else {
                    chunk
                }
            };

            if let Some(chunk) = chunk {
                let chunk_range = self.range.start..(self.range.start + chunk.len());
                self.curr_pos = tree::next_leaf(&mut self.parents, leaf);
                self.range = (self.range.start + metrics.len - leaf_start)..self.range.end;

                Some((chunk, chunk_range))
            } else {
                None
            }
        } else {
            unreachable!()
        }
    }
}

pub(crate) struct Chunks<'a>(ChunkAndRanges<'a>);

impl<'a> Chunks<'a> {
    pub(super) fn new(rope: &'a Rope, range: Range<usize>) -> Self {
        Self(ChunkAndRanges::new(rope, range))
    }

    pub(super) fn new_trim_last_terminator(rope: &'a Rope, range: Range<usize>) -> Self {
        Self(ChunkAndRanges::new_trim_last_terminator(rope, range))
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(chunk, _)| chunk)
    }
}

pub(crate) struct Lines<'a> {
    rope: &'a Rope,
    line_range: Range<usize>,
    parents: Vec<&'a Node>,
    curr_pos: Option<LeafOffset<'a>>,
    cumulative_metrics: NodeMetrics,
}

impl<'a> Lines<'a> {
    pub(crate) fn new(rope: &'a Rope, line_range: Range<usize>) -> Self {
        let mut parents = vec![];
        let (leaf_pos, cumulative_metrics) =
            tree::leaf_at_line_offset(&mut parents, rope.0.as_ref(), line_range.start);
        Self { rope, line_range, parents, cumulative_metrics, curr_pos: leaf_pos }
    }
}

impl<'a> Iterator for Lines<'a> {
    type Item = RopeLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.line_range.is_empty() {
            return None;
        }
        self.line_range = (self.line_range.start + 1)..self.line_range.end;
        if self.curr_pos.is_none() {
            let line_start = self.rope.len();
            let line_end = self.rope.len();
            return Some(RopeLine::new(self.rope, line_start..line_end));
        }

        let (leaf, leaf_start) = self.curr_pos.unwrap();
        let line_start = self.cumulative_metrics.len + leaf_start;
        if let Node::Leaf { .. } = leaf {
            let (leaf_pos, cumulative_metrics) =
                tree::next_line(&mut self.parents, leaf, leaf_start);
            self.curr_pos = leaf_pos;
            self.cumulative_metrics += &cumulative_metrics;
        } else {
            unreachable!()
        }

        // let line_end = self.cumulative_metrics.len + self.curr_pos.map(|p| p.1).unwrap_or(0);
        let line_end = match self.curr_pos {
            None => self.rope.len(),
            Some((_, o)) => self.cumulative_metrics.len + o,
        };
        Some(RopeLine::new(self.rope, line_start..line_end))
    }
}

fn trim_last_terminator(s: Option<&[u8]>) -> Option<&[u8]> {
    use bstr::ByteSlice;
    match s {
        None => None,
        Some(mut s) => {
            if s.last_byte() == Some(b'\n') {
                s = &s[..s.len() - 1];
                if s.last_byte() == Some(b'\r') {
                    s = &s[..s.len() - 1];
                }
            }
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bstr::{BStr, ByteSlice};

    use crate::rope::macros::*;
    use crate::rope::{BlockBuffer, Rope};

    #[test]
    fn lines_empty_rope() {
        let rope = Rope::empty();
        for line in rope.lines(..) {
            let mut it = line.chunks(..);
            assert_eq!(it.next(), None);
        }
    }

    #[test]
    fn chunks_empty_rope() {
        let rope = Rope::empty();
        let mut it = rope.chunks(..);
        assert_eq!(it.next(), None);
    }

    #[test]
    fn chunks_basic() {
        let mut buf = BlockBuffer::new();
        let chunks: Vec<&[u8]> = vec![
            b"This",
            b" is ",
            b"a song that",
            b" never ends.",
            b"\n",
            b"It just goes 'round ",
            b"and 'round, my friends.\n",
            b"Some people ",
            b"started singing it\n",
            b"not knowing",
            b"what it",
            b"was;\n",
            b"and they continue singing it forever",
            b" just because...\n",
        ];

        let rope = Rope(branch_b!(
            branch_b!(
                branch_b!(leaf!(buf, chunks[0]), leaf!(buf, chunks[1])),
                branch_r!(
                    branch_b!(
                        branch_r!(leaf!(buf, chunks[2]), leaf!(buf, chunks[3])),
                        leaf!(buf, chunks[4])
                    ),
                    branch_b!(leaf!(buf, chunks[5]), leaf!(buf, chunks[6])),
                ),
            ),
            branch_b!(
                branch_r!(
                    branch_b!(
                        branch_r!(leaf!(buf, chunks[7]), leaf!(buf, chunks[8])),
                        leaf!(buf, chunks[9])
                    ),
                    branch_b!(leaf!(buf, chunks[10]), leaf!(buf, chunks[11])),
                ),
                branch_b!(leaf!(buf, chunks[12]), leaf!(buf, chunks[13])),
            ),
        ));

        let it = rope.chunks(..);
        for (i, (expected, actual)) in chunks.iter().zip(it).enumerate() {
            assert_eq!(actual.as_bstr(), expected.as_bstr(), "chunk={}", i);
        }

        let lens: Vec<usize> = (std::iter::once(0).chain(chunks.iter().map(|c| c.len())))
            .scan(0, |acc, x| {
                *acc += x;
                Some(*acc)
            })
            .collect();
        let ranges = lens.iter().zip(lens.iter().skip(1)).map(|(s, e)| *s..*e);
        let expected = chunks.iter().zip(ranges);

        let it = rope.chunk_and_ranges(..);
        for (i, (expected, actual)) in expected.zip(it).enumerate() {
            assert_eq!(actual.0.as_bstr(), expected.0.as_bstr(), "chunk={}", i);
            assert_eq!(actual.1, expected.1);
        }
    }

    #[test]
    fn lines_basic() {
        let mut buf = BlockBuffer::new();
        let chunks: Vec<&[u8]> = vec![
            b"This",
            b" is ",
            b"a song that",
            b" never ends.",
            b"\n\n",
            b"It just\ngoes 'round ",
            b"and\n'round, my friends.\n",
            b"Some\npeople ",
            b"started singing it\n",
            b"not\n\nknowing",
            b" what it ",
            b"was;\n",
            b"\nand\nthey\ncontinue\nsinging\n\nit\n\nforever\n",
            b" just because...\n",
        ];

        let rope = Rope(branch_b!(
            branch_b!(
                branch_b!(leaf!(buf, chunks[0]), leaf!(buf, chunks[1])),
                branch_r!(
                    branch_b!(
                        branch_r!(leaf!(buf, chunks[2]), leaf!(buf, chunks[3])),
                        leaf!(buf, chunks[4])
                    ),
                    branch_b!(leaf!(buf, chunks[5]), leaf!(buf, chunks[6])),
                ),
            ),
            branch_b!(
                branch_r!(
                    branch_b!(
                        branch_r!(leaf!(buf, chunks[7]), leaf!(buf, chunks[8])),
                        leaf!(buf, chunks[9])
                    ),
                    branch_b!(leaf!(buf, chunks[10]), leaf!(buf, chunks[11])),
                ),
                branch_b!(leaf!(buf, chunks[12]), leaf!(buf, chunks[13])),
            ),
        ));

        let expected: Vec<Vec<&BStr>> = vec![
            vec![
                b"This".into(),
                b" is ".into(),
                b"a song that".into(),
                b" never ends.".into(),
            ],
            vec![],
            vec![b"It just".into()],
            vec![b"goes 'round ".into(), "and".into()],
            vec![b"'round, my friends.".into()],
            vec![b"Some".into()],
            vec![b"people ".into(), b"started singing it".into()],
            vec![b"not".into()],
            vec![],
            vec![b"knowing".into(), " what it ".into(), "was;".into()],
            vec![],
            vec!["and".into()],
            vec!["they".into()],
            vec!["continue".into()],
            vec!["singing".into()],
            vec![],
            vec!["it".into()],
            vec![],
            vec!["forever".into()],
            vec![" just because...".into()],
            vec![],
        ];

        let mut lineiter = rope.lines(..);
        for (linenum, expected) in expected.iter().enumerate() {
            let mut actual = Vec::with_capacity(chunks.len());
            println!("line={}", linenum);
            let line = lineiter.next().unwrap();
            for chunk in line.chunks(..) {
                actual.push(chunk.as_bstr());
            }
            assert_eq!(actual, *expected, "line={}", linenum)
        }
    }
}
