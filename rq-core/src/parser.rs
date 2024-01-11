use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

use reqwest::{Method, Version};
use std::collections::HashMap;
use std::result::Result;

use self::variables::{HashTemplateMap, TemplateString};

mod values;
mod variables;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct HttpParser;

fn http_version_from_str(input: &str) -> Version {
    match input {
        "HTTP/0.9" => Version::HTTP_09,
        "HTTP/1.0" => Version::HTTP_10,
        "HTTP/1.1" => Version::HTTP_11,
        "HTTP/2.0" => Version::HTTP_2,
        "HTTP/3.0" => Version::HTTP_3,
        _ => unreachable!(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: Method,
    pub url: TemplateString,
    pub query: HashTemplateMap,
    pub version: Version,
    pub headers: HashTemplateMap,
    pub body: TemplateString,
}

impl<'i> From<Pair<'i, Rule>> for HttpRequest {
    fn from(request: Pair<'i, Rule>) -> Self {
        let mut pairs = request.into_inner().peekable();

        let method: Method = pairs
            .next_if(|pair| pair.as_rule() == Rule::method)
            .map(|pair| pair.as_str().try_into().unwrap())
            .unwrap_or_default();

        let url = pairs.next().unwrap().into();

        let query: HashTemplateMap = pairs
            .next_if(|pair| pair.as_rule() == Rule::query)
            .map(|pair| pair.into())
            .unwrap_or_default();

        let version = pairs
            .next_if(|pair| pair.as_rule() == Rule::version)
            .map(|pair| http_version_from_str(pair.as_str()))
            .unwrap_or_default();

        let headers: HashTemplateMap = pairs
            .next_if(|pair| pair.as_rule() == Rule::headers)
            .map(|pair| pair.into())
            .unwrap_or_default();

        let body = pairs.next().map(Pair::into).unwrap_or_default();

        Self {
            method,
            url,
            query,
            version,
            headers,
            body,
        }
    }
}

#[derive(Debug)]
pub struct HttpFile {
    pub requests: Vec<HttpRequest>,
    pub variables: HashMap<String, TemplateString>,
}

impl<'i> From<Pair<'i, Rule>> for HttpFile {
    fn from(pair: Pair<Rule>) -> Self {
        let mut requests = Vec::new();
        let mut variables = HashMap::new();

        for pair in pair.into_inner() {
            match pair.as_rule() {
                Rule::request => requests.push(pair.into()),
                Rule::var_def_block => variables.extend(variables::parse_def_block(pair)),

                Rule::EOI | Rule::DELIM => (),

                _ => unreachable!(),
            }
        }

        Self {
            requests,
            variables,
        }
    }
}

pub fn parse(input: &str) -> Result<HttpFile, Box<Error<Rule>>> {
    let pair = HttpParser::parse(Rule::file, input.trim_start())?
        .next()
        .unwrap();
    Ok(HttpFile::from(pair))
}

#[cfg(test)]
mod tests {
    use core::panic;

    use crate::parser::variables::{Fragment, TemplateString};

    use super::{parse, HttpFile};
    use reqwest::{Method, Version};

    fn assert_parses(input: &str) -> HttpFile {
        let parsed = parse(input);
        match parsed {
            Ok(parsed) => parsed,
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn test_empty_input() {
        let file = assert_parses("");
        assert_eq!(file.requests.len(), 0);
    }

    #[test]
    fn test_single_requst() {
        let input = r#"
GET foo.bar HTTP/1.1

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].method, Method::GET);
        assert_eq!(file.requests[0].url.to_string(), "foo.bar");
        assert_eq!(file.requests[0].version, Version::HTTP_11);
    }

    #[test]
    fn test_optional_method() {
        let input = r#"
foo.bar HTTP/1.1

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].method, Method::default());
    }

    #[test]
    fn test_optional_version() {
        let input = r#"
GET foo.bar

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].version, Version::default());
    }

    #[test]
    fn test_var_in_url() {
        let input = r#"
GET foo{{url}}bar HTTP/1.1

"#;
        let file = assert_parses(input);
        assert_eq!(
            file.requests[0].url,
            TemplateString::new(vec![
                Fragment::raw("foo"),
                Fragment::var("url"),
                Fragment::raw("bar")
            ])
        );
    }

    #[test]
    fn test_headers() {
        let input = r#"
POST test.dev HTTP/1.0
authorization: Bearer xxxx

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].headers.len(), 1);
        assert_eq!(
            file.requests[0]
                .headers
                .get("authorization")
                .unwrap()
                .to_string(),
            "Bearer xxxx"
        );
    }

    #[test]
    fn test_var_in_headers() {
        let input = r#"
POST test.dev HTTP/1.0
aabb: {{value}}{{barbar}}

"#;
        let file = assert_parses(input);
        assert_eq!(
            file.requests[0].headers.get("aabb"),
            Some(&TemplateString::new(vec![
                Fragment::var("value"),
                Fragment::var("barbar")
            ]))
        );
    }

    #[test]
    fn test_body() {
        let input = r#"
POST test.dev HTTP/1.0

{ "test": "body" }"#;
        let file = assert_parses(input);
        assert_eq!(file.requests[0].body.to_string(), "{ \"test\": \"body\" }");
    }

    #[test]
    fn test_var_in_body() {
        let input = r#"
POST test.dev HTTP/1.0

aaa{{var}}bbb"#;
        let file = assert_parses(input);
        assert_eq!(
            file.requests[0].body,
            TemplateString::new(vec![
                Fragment::raw("aaa"),
                Fragment::var("var"),
                Fragment::raw("bbb")
            ])
        )
    }

    #[test]
    fn test_multiple_requests() {
        let input = r#"
POST test.dev HTTP/1.0
authorization: token

###

GET test.dev HTTP/1.0

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 2);
    }

    #[test]
    fn test_query_params() {
        let input = r#"
POST test.dev?foo=bar&baz=2&fif=fof HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].query.len(), 3);
        assert_eq!(
            file.requests[0].query.get("foo"),
            Some(&TemplateString::new(vec![Fragment::raw("bar")]))
        );
        assert_eq!(
            file.requests[0].query.get("baz"),
            Some(&TemplateString::new(vec![Fragment::raw("2")]))
        );
    }

    #[test]
    fn test_query_params_with_quotes() {
        let input = r#"
POST test.dev?foo=" bar"&baz='  &ciao' HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].query.len(), 2);
        assert_eq!(
            file.requests[0].query.get("foo"),
            Some(&TemplateString::raw(" bar"))
        );
        assert_eq!(
            file.requests[0].query.get("baz"),
            Some(&TemplateString::raw("  &ciao"))
        );
    }

    #[test]
    fn test_multiline_query() {
        let input = r#"
POST test.dev
        ?foo=bar
        &baz=42 HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].query.len(), 2);
        assert_eq!(
            file.requests[0].query.get("foo"),
            Some(&TemplateString::raw("bar"))
        );
        assert_eq!(
            file.requests[0].query.get("baz"),
            Some(&TemplateString::raw("42"))
        );
    }

    #[test]
    fn test_var_in_query() {
        let input = r#"
POST test.dev
        ?foo=aaa{{var}}
        &baz="bbb"{{var2}} HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(
            file.requests[0].query.get("foo"),
            Some(&TemplateString::new(vec![
                Fragment::raw("aaa"),
                Fragment::var("var")
            ]))
        );
    }

    #[test]
    fn test_file_variable() {
        let input = r#"
@name = foo
@bar = baz
@foo = " 123"

###

POST test.dev
        ?foo=bar
        &baz=42 HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(file.variables.len(), 3);
        assert_eq!(
            file.variables.get("name"),
            Some(&TemplateString::raw("foo"))
        );
        assert_eq!(file.variables.get("bar"), Some(&TemplateString::raw("baz")));
        assert_eq!(
            file.variables.get("foo"),
            Some(&TemplateString::raw(" 123"))
        );
    }

    #[test]
    fn test_var_in_file_var() {
        let input = r#"
@name = foo
@bar = aaa{{var}}
@foo = " 123"

###

POST test.dev
        ?foo=bar
        &baz=42 HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(
            file.variables.get("bar"),
            Some(&TemplateString::new(vec![
                Fragment::raw("aaa"),
                Fragment::var("var")
            ]))
        );
    }
}
