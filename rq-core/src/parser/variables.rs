use std::{collections::HashMap, fmt::Display, hash::Hash};

use pest::iterators::Pair;
use thiserror::Error;

use super::{values, Rule};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Fragment {
    Var(Variable),
    RawText(String),
}

impl Fragment {
    pub fn raw(value: &str) -> Self {
        Fragment::RawText(value.into())
    }

    pub fn var(name: &str) -> Self {
        Fragment::Var(Variable::new(name))
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct TemplateString {
    fragments: Vec<Fragment>,
}

impl TemplateString {
    pub fn new(fragments: Vec<Fragment>) -> Self {
        Self { fragments }
    }

    pub fn raw(s: &str) -> Self {
        Self {
            fragments: vec![Fragment::RawText(s.into())],
        }
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

impl From<Pair<'_, Rule>> for TemplateString {
    fn from(value: Pair<'_, Rule>) -> Self {
        let inner = value.into_inner();

        let fragments = inner
            .map(|pair| match pair.as_rule() {
                Rule::var => {
                    let var_name = pair.into_inner().next().unwrap().as_str();
                    Fragment::var(var_name)
                }
                _ => Fragment::raw(values::unquote(pair.as_str())),
            })
            .collect::<Vec<_>>();

        Self::new(fragments)
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

pub fn parse_def_block(var_def_block: Pair<Rule>) -> HashMap<String, TemplateString> {
    var_def_block
        .into_inner()
        .map(|var_def| {
            let mut pairs = var_def.into_inner();

            let name = pairs.next().unwrap().as_str().to_string();
            let value = pairs.next().unwrap().into();

            (name, value)
        })
        .collect()
}
