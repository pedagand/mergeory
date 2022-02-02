use crate::generic_tree::{NodeKind, Subtree, Token, Tree};
use crate::tree_formatter::{TreeFormattable, TreeFormatter};
use tree_sitter::Parser;

pub struct SynNode<'t>(pub Tree<'t, Subtree<SynNode<'t>>>);

impl<'t> TreeFormattable for SynNode<'t> {
    fn write_with<F: TreeFormatter>(&self, fmt: &mut F) -> std::io::Result<()> {
        self.0.write_with(fmt)
    }
}

fn build_syn_tree<'t>(
    cursor: &mut tree_sitter::TreeCursor,
    source: &'t [u8],
    ignore_whitespace: bool,
    root: bool,
) -> SynNode<'t> {
    let node = cursor.node();
    if !node.is_named() {
        return SynNode(Tree::Leaf(Token::new(
            &source[node.byte_range()],
            ignore_whitespace,
        )));
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
                    node: SynNode(Tree::Leaf(Token::new(
                        &source[cur_byte..start_byte],
                        ignore_whitespace,
                    ))),
                });
            }
            let subtree = build_syn_tree(cursor, source, ignore_whitespace, false);
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
            node: SynNode(Tree::Leaf(Token::new(
                &source[cur_byte..end_byte],
                ignore_whitespace,
            ))),
        })
    }
    SynNode(Tree::Node(kind, children))
}

pub fn parse_source<'t>(
    source: &'t [u8],
    parser: &mut Parser,
    ignore_whitespace: bool,
) -> Option<SynNode<'t>> {
    parser.reset();
    let tree = parser.parse(source, None)?;
    let syn_tree = build_syn_tree(&mut tree.walk(), source, ignore_whitespace, true);
    Some(syn_tree)
}

const EXTRA_BLOCK: NodeKind = NodeKind::MAX - 2;

fn finalize_last_extra_block(child_list: &mut Vec<Subtree<SynNode>>) {
    let final_leaves = match child_list.last_mut() {
        Some(Subtree {
            node: SynNode(Tree::Node(kind, sub_children)),
            ..
        }) => {
            debug_assert!(*kind == EXTRA_BLOCK);
            let mut first_final_leaf = 0;
            for (index, child) in sub_children.iter().enumerate().rev() {
                if !matches!(&child.node.0, Tree::Leaf(_)) {
                    first_final_leaf = index + 1;
                    break;
                }
            }
            sub_children.drain(first_final_leaf..).collect()
        }
        _ => Vec::new(),
    };
    child_list.extend(final_leaves);
}

pub fn add_extra_blocks<'t>(tree: &SynNode<'t>) -> SynNode<'t> {
    SynNode(tree.0.convert(|children| {
        let mut child_list = Vec::new();
        for child in children {
            match (&child.node.0, child_list.last_mut()) {
                (
                    Tree::Leaf(_),
                    None
                    | Some(Subtree {
                        node: SynNode(Tree::Leaf(_)),
                        ..
                    }),
                ) => {
                    finalize_last_extra_block(&mut child_list);
                    child_list.push(child.as_ref().map(add_extra_blocks))
                }
                (
                    Tree::Leaf(tok),
                    Some(Subtree {
                        node: SynNode(Tree::Node(kind, sub_children)),
                        ..
                    }),
                ) => {
                    debug_assert!(*kind == EXTRA_BLOCK);
                    if tok.is_extra_block_separator() {
                        finalize_last_extra_block(&mut child_list);
                        child_list.push(child.as_ref().map(add_extra_blocks));
                    } else {
                        sub_children.push(child.as_ref().map(add_extra_blocks));
                    }
                }
                (
                    Tree::Node(_, _),
                    Some(Subtree {
                        field,
                        node: SynNode(Tree::Node(kind, sub_children)),
                    }),
                ) if *field == child.field => {
                    debug_assert!(*kind == EXTRA_BLOCK);
                    sub_children.push(child.as_ref().map(add_extra_blocks))
                }
                _ => {
                    finalize_last_extra_block(&mut child_list);
                    child_list.push(Subtree {
                        field: child.field,
                        node: SynNode(Tree::Node(
                            EXTRA_BLOCK,
                            vec![child.as_ref().map(add_extra_blocks)],
                        )),
                    })
                }
            }
        }
        finalize_last_extra_block(&mut child_list);
        child_list
    }))
}
