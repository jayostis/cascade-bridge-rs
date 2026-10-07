use crate::error::Result;

pub(crate) mod json_schema;
pub(crate) mod xsd;

#[cfg(test)]
mod cda;

pub(crate) trait Schema: Send {
    fn errors(&self, text: &str) -> Result<Vec<SchemaFinding>>;
}

pub(crate) struct SchemaFinding {
    body: String,
    within: Option<String>,
}

impl SchemaFinding {
    pub(crate) fn body(&self) -> &str {
        &self.body
    }

    /// Relative to what was validated; none where it is that itself.
    pub(crate) fn within(&self) -> Option<&str> {
        self.within.as_deref()
    }
}
