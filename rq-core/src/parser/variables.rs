use std::{collections::HashMap, fmt::Display, hash::Hash, ops::Deref, str::FromStr};

use pest::{iterators::Pair, Parser};
use thiserror::Error;

use super::{values, HttpParser, Rule};

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

    pub fn fill(&self, parameters: &HashMap<String, TemplateString>) -> Result<String, FillError> {
        self.fragments
            .iter()
            .map(|fragment| {
                let s = match fragment {
                    Fragment::Var(v) => parameters
                        .get(&v.name)
                        .ok_or(FillError::from(v.clone()))
                        .and_then(|s| s.fill(parameters))?,
                    Fragment::RawText(s) => s.to_owned(),
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

impl FromStr for TemplateString {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HttpParser::parse(Rule::var_def_value, s)
            .map_err(|e| e.to_string())?
            .next()
            .unwrap()
            .into())
    }
}

#[derive(Debug, Error, PartialEq)]
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

        if s.starts_with(' ') | s.ends_with(' ') {
            return write!(f, "\"{s}\"");
        }

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

#[derive(Debug, Clone, Default)]
pub struct HashTemplateMap(HashMap<String, TemplateString>);

impl HashTemplateMap {
    pub fn fill(
        &self,
        params: &HashMap<String, TemplateString>,
    ) -> Result<HashMap<String, String>, FillError> {
        let filled = self
            .0
            .iter()
            .map(|(k, v)| {
                let v = v.fill(params)?;

                Ok((k.to_owned(), v))
            })
            .collect::<Result<HashMap<_, _>, FillError>>()?;

        Ok(filled)
    }
}

impl Deref for HashTemplateMap {
    type Target = HashMap<String, TemplateString>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Pair<'_, Rule>> for HashTemplateMap {
    fn from(value: Pair<'_, Rule>) -> Self {
        let headers = value
            .into_inner()
            .map(|pair| {
                let mut kv = pair.into_inner();
                let key = kv.next().unwrap().as_str().to_string();
                let value = kv.next().unwrap().into();

                (key, value)
            })
            .collect();

        Self(headers)
    }
}

#[cfg(test)]
mod tests {
    mod variable {}

    mod template_string {
        use std::collections::HashMap;

        use crate::parser::variables::{FillError, Fragment, TemplateString, Variable};

        #[test]
        fn test_display() {
            let ts = TemplateString::new(vec![Fragment::var("foo")]);
            let ts2 = TemplateString::raw("barbar");
            let ts_quoted = TemplateString::raw("  baz  ");

            assert_eq!(ts.to_string(), "{{foo}}");
            assert_eq!(ts2.to_string(), "barbar");
            assert_eq!(ts_quoted.to_string(), "\"  baz  \"");
        }

        #[test]
        fn test_parse_str() {
            let s = "' foo'{{bar}}baz";
            let expected = TemplateString::new(vec![
                Fragment::raw(" foo"),
                Fragment::var("bar"),
                Fragment::raw("baz"),
            ]);

            assert_eq!(s.parse::<TemplateString>().unwrap(), expected);
        }

        #[test]
        fn test_fill() {
            let ts = TemplateString::new(vec![
                Fragment::raw(" foo"),
                Fragment::var("bar"),
                Fragment::raw("baz"),
            ]);
            let ts2 = TemplateString::raw("foobarbaz");
            let ts3 = TemplateString::new(vec![Fragment::var("baz")]);
            let values =
                HashMap::from([("bar".into(), "FOOBAR".parse::<TemplateString>().unwrap())]);

            assert_eq!(ts.fill(&values).unwrap(), " fooFOOBARbaz");
            assert_eq!(ts2.fill(&values).unwrap(), "foobarbaz");
            assert_eq!(
                ts3.fill(&values),
                Err(FillError::from(Variable::new("baz")))
            )
        }

        #[test]
        fn test_is_empty() {
            let ts = TemplateString::new(vec![]);
            let ts2 = TemplateString::raw("");
            let ts3 = TemplateString::new(vec![Fragment::raw(""), Fragment::raw("")]);

            assert!(ts.is_empty());
            assert!(ts2.is_empty());
            assert!(ts3.is_empty());
        }
    }
}
