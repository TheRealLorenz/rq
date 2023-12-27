use std::{collections::HashMap, fmt::Display};

use pest::iterators::Pair;
use thiserror::Error;

use super::Rule;

#[derive(Debug, Clone)]
pub struct Variable {
    name: String,
}

impl Variable {
    pub fn new(name: &str) -> Self {
        Variable {
            name: name.to_owned(),
        }
    }
}

impl Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // i.e. if self.name = 'foo', this outputs '{{foo}}'
        write!(f, "{{{{{}}}}}", self.name)
    }
}

#[derive(Debug, Clone)]
pub enum Fragment {
    Var(Variable),
    RawText(String),
}

#[derive(Debug, Clone, Default)]
pub struct TemplateString {
    fragments: Vec<Fragment>,
}

impl TemplateString {
    pub fn new(fragments: Vec<Fragment>) -> Self {
        Self { fragments }
    }

    pub fn fill(&self, parameters: &HashMap<String, String>) -> Result<String, FillError> {
        self.fragments
            .iter()
            .map(|fragment| {
                let s = match fragment {
                    Fragment::Var(v) => parameters
                        .get(&v.name)
                        .map(|s| s.as_str())
                        .ok_or(v.clone())?,
                    Fragment::RawText(s) => s.as_str(),
                };

                Ok(s)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
            || self.fragments.iter().all(|fragment| match fragment {
                Fragment::Var(_) => false,
                Fragment::RawText(s) => s.is_empty(),
            })
    }
}

#[derive(Debug, Error)]
#[error("missing field '{}'", .missing_variable.name)]
pub struct FillError {
    missing_variable: Variable,
}

impl From<Variable> for FillError {
    fn from(value: Variable) -> Self {
        FillError {
            missing_variable: value,
        }
    }
}

impl Display for TemplateString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .fragments
            .iter()
            .map(|fragment| match fragment {
                Fragment::Var(v) => v.to_string(),
                Fragment::RawText(s) => s.to_owned(),
            })
            .collect::<String>();

        write!(f, "{s}")
    }
}

pub fn parse(var_def_block: Pair<Rule>) -> HashMap<String, String> {
    var_def_block
        .into_inner()
        .map(|var_def| {
            let mut pairs = var_def.into_inner();

            let name = pairs.next().unwrap().as_str().to_string();
            let value = pairs.next().unwrap().as_str().to_string();

            (name, value)
        })
        .collect()
}
