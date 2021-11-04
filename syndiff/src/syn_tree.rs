use crate::generic_tree::{Subtree, Tree, WriteTree};
use tree_sitter::Parser;

pub struct SynNode<'t>(pub Tree<'t, Subtree<SynNode<'t>>>);

impl<'t> WriteTree for SynNode<'t> {
    fn write_tree<O: std::io::Write>(&self, output: &mut O) -> std::io::Result<()> {
        self.0.write_tree(output)
    }
}

fn build_syn_tree<'t>(cursor: &mut tree_sitter::TreeCursor, source: &'t [u8]) -> SynNode<'t> {
    let node = cursor.node();
    if !node.is_named() {
        return SynNode(Tree::Leaf(&source[node.byte_range()]));
    }
    let kind = node.kind_id();
    let mut children = Vec::new();
    let mut cur_byte = node.start_byte();
    if cursor.goto_first_child() {
        loop {
            let field = cursor.field_id();
            let start_byte = cursor.node().start_byte();
            if cur_byte < start_byte {
                // Never loose any byte by recreating leaf node
                children.push(Subtree {
                    field: None,
                    node: SynNode(Tree::Leaf(&source[cur_byte..start_byte])),
                });
            }
            let subtree = build_syn_tree(cursor, source);
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
    if cur_byte < node.end_byte() {
        children.push(Subtree {
            field: None,
            node: SynNode(Tree::Leaf(&source[cur_byte..node.end_byte()])),
        })
    }
    SynNode(Tree::Node(kind, children))
}

pub fn parse_source<'t>(source: &'t [u8], parser: &mut Parser) -> Option<SynNode<'t>> {
    parser.reset();
    let tree = parser.parse(source, None)?;
    let syn_tree = build_syn_tree(&mut tree.walk(), source);
    Some(syn_tree)
}
