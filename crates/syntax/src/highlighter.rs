use tree_sitter as ts;

use crate::Language;
use editor::BufferContents;

#[tracing::instrument(skip_all)]
pub fn highlight(
    buffer: &BufferContents,
    language: Language,
    tree: ts::Tree,
) -> editor::Highlights {
    let query = ts::Query::new(language.ts, &language.highlight_query).expect("invalid query");
    let mut cursor = ts::QueryCursor::new();
    let mut highlights = iset::IntervalMap::new();
    let captures =
        cursor.captures(&query, tree.root_node(), crate::BufferContentsTextProvider(buffer));
    for (query_match, _) in captures {
        for capture in query_match.captures {
            let capture_name = &query.capture_names()[capture.index as usize];
            let capture_range = capture.node.byte_range();
            highlights.insert(capture_range, capture_name.clone());
        }
    }
    highlights
}
