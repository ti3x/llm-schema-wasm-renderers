//! The closed catalog of allowed components. Anything outside this list
//! is rejected at parse time — there is no way for a spec to address an
//! arbitrary HTML tag, an unknown component, or a user-supplied template.

use crate::error::{RenderError, RenderResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Text,
    Heading,
    Link,
    Image,
    Container,
    Card,
    List,
    Table,
    Button,
    Input,
}

impl Tag {
    pub fn from_name(name: &str, path: &str) -> RenderResult<Self> {
        Ok(match name {
            "Text" => Tag::Text,
            "Heading" => Tag::Heading,
            "Link" => Tag::Link,
            "Image" => Tag::Image,
            "Container" => Tag::Container,
            "Card" => Tag::Card,
            "List" => Tag::List,
            "Table" => Tag::Table,
            "Button" => Tag::Button,
            "Input" => Tag::Input,
            _ => {
                return Err(RenderError::Spec {
                    path: path.into(),
                    msg: format!("unknown component `{name}`"),
                })
            }
        })
    }

    /// Whether this component can have a `children` array.
    pub fn accepts_children(self) -> bool {
        matches!(self, Tag::Container | Tag::Card | Tag::List)
    }
}
