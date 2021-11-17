use crate::generic_tree::{Subtree, Token, Tree};
use tree_sitter::Parser;

pub struct SynNode<'t>(pub Tree<'t, Subtree<SynNode<'t>>>);

impl<'t> SynNode<'t> {
    pub fn write_to(&self, output: &mut impl std::io::Write) -> std::io::Result<()> {
        self.0.write_to(output, |ch, out| ch.node.write_to(out))
    }
}

fn build_syn_tree<'t>(
    cursor: &mut tree_sitter::TreeCursor,
    source: &'t [u8],
    root: bool,
) -> SynNode<'t> {
    let node = cursor.node();
    if !node.is_named() {
        return SynNode(Tree::Leaf(Token::new(&source[node.byte_range()])));
    }
    let kind = node.kind_id();
    let mut children = Vec::new();

    // We cannot trust tree-sitter root node span, that is shorter than full file if the file
    // starts with an ignored leaf (like space or new line).
    let mut cur_byte = if root { 0 } else { node.start_byte() };

    if cursor.goto_first_child() {
        loop {
            let field = cursor.field_id();
            let start_byte = cursor.node().start_byte();
            if cur_byte < start_byte {
                // Never loose any byte by recreating leaf node
                children.push(Subtree {
                    field: None,
                    node: SynNode(Tree::Leaf(Token::new(&source[cur_byte..start_byte]))),
                });
            }
            let subtree = build_syn_tree(cursor, source, false);
            children.push(Subtree {
                field,
                node: subtree,
            });
            cur_byte = cursor.node().end_byte();
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        assert!(cursor.goto_parent());
    }

    let end_byte = if root { source.len() } else { node.end_byte() };
    if cur_byte < end_byte {
        children.push(Subtree {
            field: None,
            node: SynNode(Tree::Leaf(Token::new(&source[cur_byte..end_byte]))),
        })
    }
    SynNode(Tree::Node(kind, children))
}

pub fn parse_source<'t>(source: &'t [u8], parser: &mut Parser) -> Option<SynNode<'t>> {
    parser.reset();
    let tree = parser.parse(source, None)?;
    let syn_tree = build_syn_tree(&mut tree.walk(), source, true);
    Some(syn_tree)
}
