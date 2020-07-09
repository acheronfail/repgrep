use crate::model::Item;

#[derive(Debug)]
pub struct ReplacementCriteria {
    pub items: Vec<Item>,
    pub text: String,
    pub encoding: Option<String>,
}

impl ReplacementCriteria {
    pub fn new<S: AsRef<str>>(text: S, items: Vec<Item>) -> ReplacementCriteria {
        let text = text.as_ref().to_owned();
        ReplacementCriteria {
            text,
            items,
            encoding: None,
        }
    }

    pub fn set_encoding(&mut self, encoding: impl AsRef<str>) {
        self.encoding = Some(encoding.as_ref().to_owned());
    }
}
