//! Validated, platform-independent version-1 analysis trees.
//! Handles are indices in the original node array and remain stable for a tree's lifetime.

use crate::board::Board;
use crate::puzzle::is_uci_move;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeHandle(usize);

impl NodeHandle {
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoveRole {
    Main,
    Alternative,
    Sideline,
}

#[derive(Debug)]
pub struct AnalysisMove {
    pub uci: String,
    pub san: String,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ContentSpan {
    Text(String),
    MoveRef {
        node: NodeHandle,
        label: Option<String>,
    },
}

#[derive(Debug)]
pub struct AnalysisNode {
    id: String,
    parent: Option<NodeHandle>,
    children: Vec<NodeHandle>,
    fen: String,
    movement: Option<AnalysisMove>,
    role: Option<MoveRole>,
    comment: String,
    content: Vec<ContentSpan>,
    nags: Vec<u64>,
}

impl AnalysisNode {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn parent(&self) -> Option<NodeHandle> {
        self.parent
    }

    pub fn children(&self) -> &[NodeHandle] {
        &self.children
    }

    pub fn fen(&self) -> &str {
        &self.fen
    }

    pub fn movement(&self) -> Option<&AnalysisMove> {
        self.movement.as_ref()
    }

    /// None for the root; every non-root node has a role.
    pub fn role(&self) -> Option<MoveRole> {
        self.role
    }

    pub fn comment(&self) -> &str {
        &self.comment
    }

    pub fn content(&self) -> &[ContentSpan] {
        &self.content
    }

    pub fn nags(&self) -> &[u64] {
        &self.nags
    }
}

#[derive(Debug)]
pub struct AnalysisTree {
    root: NodeHandle,
    nodes: Vec<AnalysisNode>,
    by_id: HashMap<String, NodeHandle>,
}

impl AnalysisTree {
    pub fn root(&self) -> NodeHandle {
        self.root
    }

    pub fn nodes(&self) -> &[AnalysisNode] {
        &self.nodes
    }

    pub fn node(&self, handle: NodeHandle) -> Option<&AnalysisNode> {
        self.nodes.get(handle.0)
    }

    pub fn by_id(&self, id: &str) -> Option<NodeHandle> {
        self.by_id.get(id).copied()
    }

    /// Validate links and the projected main line before resolving any prose references.
    pub(crate) fn parse(
        value: &Value,
        puzzle_fen: &str,
        solution: &[String],
    ) -> Result<Self, String> {
        let data = object(value, "analysis")?;
        if data.get("version").and_then(Value::as_u64) != Some(1) {
            return Err("analysis.version must be integer 1".into());
        }
        let root_id = nonempty_string(data, "root", "analysis")?;
        let raw_nodes = data
            .get("nodes")
            .and_then(Value::as_array)
            .filter(|nodes| !nodes.is_empty())
            .ok_or("analysis.nodes must be a non-empty array")?;

        let mut nodes = Vec::with_capacity(raw_nodes.len());
        let mut by_id = HashMap::with_capacity(raw_nodes.len());
        let mut parent_ids = Vec::with_capacity(raw_nodes.len());
        let mut child_ids = Vec::with_capacity(raw_nodes.len());
        let mut raw_contents = Vec::with_capacity(raw_nodes.len());

        for (index, raw) in raw_nodes.iter().enumerate() {
            let where_ = format!("analysis.nodes[{index}]");
            let data = object(raw, &where_)?;
            let id = nonempty_string(data, "id", &where_)?.to_owned();
            let handle = NodeHandle(index);
            if by_id.insert(id.clone(), handle).is_some() {
                return Err(format!("{where_}: duplicate node id {id:?}"));
            }

            let fen = required_string(data, "fen", &where_)?;
            Board::from_fen(fen)
                .map_err(|error| format!("{where_} ({id}): invalid FEN: {error}"))?;
            let is_root = id == root_id;
            if is_root && fen != puzzle_fen {
                return Err(format!("{where_} ({id}): root FEN differs from puzzle FEN"));
            }

            let (parent_id, movement, role, nags) = if is_root {
                for forbidden in ["parent", "move", "role", "nags"] {
                    if data.contains_key(forbidden) {
                        return Err(format!("{where_} ({id}): root must not have {forbidden}"));
                    }
                }
                (None, None, None, Vec::new())
            } else {
                let parent = nonempty_string(data, "parent", &where_)?.to_owned();
                let movement = object(
                    data.get("move")
                        .ok_or_else(|| format!("{where_} ({id}): missing move"))?,
                    &format!("{where_} ({id}).move"),
                )?;
                if movement.len() != 2
                    || !movement.contains_key("uci")
                    || !movement.contains_key("san")
                {
                    return Err(format!(
                        "{where_} ({id}): move must contain exactly uci and san"
                    ));
                }
                let uci = required_string(movement, "uci", &where_)?;
                if !is_uci_move(uci) {
                    return Err(format!("{where_} ({id}): invalid move.uci {uci:?}"));
                }
                let san = nonempty_string(movement, "san", &where_)?;
                let role = match data.get("role") {
                    None => MoveRole::Sideline,
                    Some(Value::String(role)) if role == "main" => MoveRole::Main,
                    Some(Value::String(role)) if role == "alternative" => MoveRole::Alternative,
                    Some(Value::String(role)) if role == "sideline" => MoveRole::Sideline,
                    _ => return Err(format!("{where_} ({id}): invalid role")),
                };
                let nags = match data.get("nags") {
                    None => Vec::new(),
                    Some(Value::Array(numbers)) => numbers
                        .iter()
                        .map(|number| {
                            number.as_u64().ok_or_else(|| {
                                format!("{where_} ({id}): NAGs must be non-negative integers")
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    _ => return Err(format!("{where_} ({id}): nags must be an array")),
                };
                (
                    Some(parent),
                    Some(AnalysisMove {
                        uci: uci.to_owned(),
                        san: san.to_owned(),
                    }),
                    Some(role),
                    nags,
                )
            };

            let comment = match data.get("comment") {
                None => "",
                Some(Value::String(comment)) => comment.as_str(),
                _ => return Err(format!("{where_} ({id}): comment must be a string")),
            };
            let children = match data.get("children") {
                None => Vec::new(),
                Some(Value::Array(children)) => children
                    .iter()
                    .map(|child| {
                        child
                            .as_str()
                            .filter(|id| !id.trim().is_empty())
                            .map(str::to_owned)
                            .ok_or_else(|| format!("{where_} ({id}): invalid child ID"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(format!("{where_} ({id}): children must be an array")),
            };
            nodes.push(AnalysisNode {
                id,
                parent: None,
                children: Vec::new(),
                fen: fen.to_owned(),
                movement,
                role,
                comment: comment.to_owned(),
                content: Vec::new(),
                nags,
            });
            parent_ids.push(parent_id);
            child_ids.push(children);
            raw_contents.push(data.get("content"));
        }

        let root = *by_id
            .get(root_id)
            .ok_or_else(|| format!("analysis.root {root_id:?} does not name a node"))?;
        for (index, parent_id) in parent_ids.iter().enumerate() {
            if let Some(parent_id) = parent_id {
                let parent = *by_id.get(parent_id).ok_or_else(|| {
                    format!("node {:?}: unknown parent {parent_id:?}", nodes[index].id)
                })?;
                nodes[index].parent = Some(parent);
            } else if index != root.0 {
                return Err(format!("node {:?}: missing parent", nodes[index].id));
            }
        }

        let mut inbound = vec![0usize; nodes.len()];
        for (index, children) in child_ids.iter().enumerate() {
            let mut unique = HashSet::new();
            let mut handles = Vec::with_capacity(children.len());
            for child_id in children {
                let child = *by_id.get(child_id).ok_or_else(|| {
                    format!("node {:?}: missing child {child_id:?}", nodes[index].id)
                })?;
                if child == root {
                    return Err(format!(
                        "node {:?}: root cannot be a child",
                        nodes[index].id
                    ));
                }
                if !unique.insert(child) || inbound[child.0] != 0 {
                    return Err(format!(
                        "node {:?}: duplicate/multiple-parent child {child_id:?}",
                        nodes[index].id
                    ));
                }
                if nodes[child.0].parent != Some(NodeHandle(index)) {
                    return Err(format!(
                        "node {:?}: parent/child mismatch for {child_id:?}",
                        nodes[index].id
                    ));
                }
                inbound[child.0] += 1;
                handles.push(child);
            }
            nodes[index].children = handles;
        }
        for (index, node) in nodes.iter().enumerate() {
            if index != root.0 && inbound[index] != 1 {
                return Err(format!("node {:?}: missing parent-to-child link", node.id));
            }
        }

        let mut visited = HashSet::new();
        let mut stack = vec![root];
        while let Some(handle) = stack.pop() {
            if !visited.insert(handle) {
                return Err(format!(
                    "analysis: repeated/cyclic node {:?}",
                    nodes[handle.0].id
                ));
            }
            stack.extend(nodes[handle.0].children.iter().copied());
        }
        if visited.len() != nodes.len() {
            return Err("analysis: disconnected or cyclic nodes".into());
        }

        let mut main_path = HashSet::new();
        let mut projected = Vec::new();
        let mut current = root;
        loop {
            let mut next = None;
            for child in &nodes[current.0].children {
                if nodes[child.0].role == Some(MoveRole::Main) {
                    if next.replace(*child).is_some() {
                        return Err(format!(
                            "node {:?}: multiple main children",
                            nodes[current.0].id
                        ));
                    }
                }
            }
            let Some(child) = next else { break };
            main_path.insert(child);
            projected.push(nodes[child.0].movement.as_ref().unwrap().uci.as_str());
            current = child;
        }
        if nodes.iter().enumerate().any(|(index, node)| {
            node.role == Some(MoveRole::Main) && !main_path.contains(&NodeHandle(index))
        }) {
            return Err("analysis: main node outside the projected main path".into());
        }
        if projected != solution.iter().map(String::as_str).collect::<Vec<_>>() {
            return Err(format!(
                "analysis: projected main line {projected:?} does not match solution {solution:?}"
            ));
        }

        let mut tree = Self { root, nodes, by_id };
        for (index, raw_content) in raw_contents.iter().enumerate() {
            let context = format!("analysis node {:?}", tree.nodes[index].id);
            let spans = parse_content(
                *raw_content,
                &tree.nodes[index].comment,
                Some(&tree),
                &context,
            )?;
            tree.nodes[index].content = spans;
        }
        Ok(tree)
    }
}

/// Explicitly tagged spans only. Plain text is never inferred to be a move reference.
pub(crate) fn parse_content(
    value: Option<&Value>,
    fallback: &str,
    tree: Option<&AnalysisTree>,
    context: &str,
) -> Result<Vec<ContentSpan>, String> {
    let Some(value) = value else {
        return Ok(if fallback.is_empty() {
            Vec::new()
        } else {
            vec![ContentSpan::Text(fallback.to_owned())]
        });
    };
    let spans = value
        .as_array()
        .ok_or_else(|| format!("{context}: content must be an array"))?;
    spans
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let where_ = format!("{context} span {index}");
            let data = object(raw, &where_)?;
            match data.get("type").and_then(Value::as_str) {
                Some("text") => {
                    if data.len() != 2 || !data.contains_key("text") {
                        return Err(format!(
                            "{where_}: text span must have exactly type and text"
                        ));
                    }
                    Ok(ContentSpan::Text(
                        nonempty_string(data, "text", &where_)?.to_owned(),
                    ))
                }
                Some("move_ref") => {
                    if data.len() != (if data.contains_key("label") { 3 } else { 2 })
                        || !data.contains_key("node")
                    {
                        return Err(format!("{where_}: invalid move_ref keys"));
                    }
                    let id = nonempty_string(data, "node", &where_)?;
                    let tree = tree
                        .ok_or_else(|| format!("{where_}: move_ref requires an analysis tree"))?;
                    let node = tree
                        .by_id(id)
                        .ok_or_else(|| format!("{where_}: dangling move_ref {id:?}"))?;
                    if node == tree.root {
                        return Err(format!("{where_}: move_ref cannot target the root"));
                    }
                    let label = if data.contains_key("label") {
                        Some(nonempty_string(data, "label", &where_)?.to_owned())
                    } else {
                        None
                    };
                    Ok(ContentSpan::MoveRef { node, label })
                }
                _ => Err(format!("{where_}: unknown span type")),
            }
        })
        .collect()
}

fn object<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{context}: expected object"))
}

fn required_string<'a>(
    data: &'a Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<&'a str, String> {
    data.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{context}: {field} must be a string"))
}

fn nonempty_string<'a>(
    data: &'a Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<&'a str, String> {
    let value = required_string(data, field, context)?;
    if value.trim().is_empty() {
        Err(format!("{context}: {field} must not be empty"))
    } else {
        Ok(value)
    }
}
