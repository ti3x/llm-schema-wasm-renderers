use crate::catalog::Tag;
use crate::expr::Binding;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum PropValue {
    /// A literal JSON value baked into the spec.
    Lit(Value),
    /// A binding to be resolved at render time.
    Bind(Binding),
}

#[derive(Debug, Clone)]
pub enum Node {
    Element {
        tag: Tag,
        props: HashMap<String, PropValue>,
        children: Vec<Node>,
    },
    /// A bare string in the `children` array becomes a literal text node.
    Text(String),
}
