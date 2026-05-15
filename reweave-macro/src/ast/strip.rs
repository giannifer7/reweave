use super::ASTError;
use crate::parser::Parser;
use crate::types::NodeKind;

pub fn strip_space_before_comments(
    content: &[u8],
    parser: &mut Parser,
    node_idx: usize,
) -> Result<(), ASTError> {
    let mut to_remove: Vec<usize> = Vec::new();
    let mut spaces_to_strip: Vec<usize> = Vec::new();

    // Analysis phase: walk forward; when we hit a comment, walk back over
    // all consecutive Space nodes preceding it.
    {
        let node = parser
            .get_node(node_idx)
            .ok_or(ASTError::NodeNotFound(node_idx))?;

        let mut i = 0;
        while i < node.parts.len() {
            let part_idx = node.parts[i];
            let part = parser
                .get_node(part_idx)
                .ok_or(ASTError::NodeNotFound(part_idx))?;

            let is_line_comment = part.kind == NodeKind::LineComment;
            let is_block_comment = part.kind == NodeKind::BlockComment;

            if is_line_comment || is_block_comment {
                let block_comment_newline = if is_block_comment {
                    is_followed_by_newline(content, parser, part_idx)?
                } else {
                    false
                };

                if is_line_comment || block_comment_newline {
                    // Walk back over ALL consecutive Space nodes.
                    let mut j = i;
                    while j > 0 {
                        let prev_idx = node.parts[j - 1];
                        let prev = parser
                            .get_node(prev_idx)
                            .ok_or(ASTError::NodeNotFound(prev_idx))?;
                        if prev.kind == NodeKind::Space {
                            to_remove.push(j - 1);
                            j -= 1;
                        } else {
                            // Not a Space — trim trailing spaces from a
                            // preceding Text node, then stop.
                            if prev.kind == NodeKind::Text {
                                spaces_to_strip.push(prev_idx);
                            }
                            break;
                        }
                    }
                }
            }
            i += 1;
        }
    }

    // Modification phase
    if !to_remove.is_empty() {
        // De-duplicate (a single Space may be adjacent to two comments).
        to_remove.sort_unstable();
        to_remove.dedup();
        let node = parser
            .get_node_mut(node_idx)
            .ok_or(ASTError::NodeNotFound(node_idx))?;
        for &idx in to_remove.iter().rev() {
            node.parts.remove(idx);
        }
    }

    for idx in spaces_to_strip {
        parser.strip_ending_space(content, idx)?;
    }

    // Recurse into children (re-read after modification to skip removed nodes)
    let children: Vec<usize> = parser
        .get_node(node_idx)
        .ok_or(ASTError::NodeNotFound(node_idx))?
        .parts
        .clone();
    for child_idx in children {
        strip_space_before_comments(content, parser, child_idx)?;
    }

    Ok(())
}

fn is_followed_by_newline(
    content: &[u8],
    parser: &Parser,
    node_idx: usize,
) -> Result<bool, ASTError> {
    let node = parser
        .get_node(node_idx)
        .ok_or(ASTError::NodeNotFound(node_idx))?;
    let end_pos = node.end_pos;

    Ok(end_pos < content.len() && content[end_pos] == b'\n')
}
