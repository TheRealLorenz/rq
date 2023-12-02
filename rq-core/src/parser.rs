use pest::error::Error;
use pest::iterators::{Pair, Pairs};
use pest::Parser;

use reqwest::header::HeaderMap;
use reqwest::{Method, Version};
use std::collections::HashMap;
use std::fmt::Display;
use std::result::Result;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct HttpParser;

#[derive(Clone, Debug, Default)]
struct HttpHeaders(HashMap<String, String>);

impl Display for HttpHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .0
            .iter()
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join(", ");

        write!(f, "[{s}]")
    }
}

impl<'i> From<Pairs<'i, Rule>> for HttpHeaders {
    fn from(pairs: Pairs<'i, Rule>) -> Self {
        let headers = pairs
            .map(|pair| {
                let mut kv = pair.into_inner();
                let key = kv.next().unwrap().as_str().to_string();
                let value = kv.next().unwrap().as_str().to_string();

                (key, value)
            })
            .collect();

        HttpHeaders(headers)
    }
}

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
    pub url: String,
    pub query: HashMap<String, String>,
    pub version: Version,
    headers: HttpHeaders,
    pub body: String,
}

impl HttpRequest {
    pub fn headers(&self) -> HeaderMap {
        (&self.headers.0).try_into().unwrap()
    }
}

impl<'i> From<Pair<'i, Rule>> for HttpRequest {
    fn from(request: Pair<'i, Rule>) -> Self {
        let mut pairs = request.into_inner().peekable();

        let method: Method = pairs
            .next_if(|pair| pair.as_rule() == Rule::method)
            .map(|pair| pair.as_str().try_into().unwrap())
            .unwrap_or_default();

        let url = pairs.next().unwrap().as_str().to_string();

        let query = pairs
            .next_if(|pair| pair.as_rule() == Rule::query)
            .map(|pair| {
                pair.into_inner()
                    .map(|pair| {
                        let mut pairs = pair.into_inner();

                        let key = pairs.next().unwrap().as_str().to_string();
                        let value = pairs.next().unwrap().as_str().to_string();

                        (key, value)
                    })
                    .collect::<HashMap<String, String>>()
            })
            .unwrap_or_default();

        let version = pairs
            .next_if(|pair| pair.as_rule() == Rule::version)
            .map(|pair| http_version_from_str(pair.as_str()))
            .unwrap_or_default();

        let headers: HttpHeaders = pairs
            .next_if(|pair| pair.as_rule() == Rule::headers)
            .map(|pair| pair.into_inner().into())
            .unwrap_or_default();

        let body = pairs
            .next()
            .map(|pair| pair.as_str().to_string())
            .unwrap_or_default();

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

impl Display for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {:?} {}",
            self.method, self.url, self.version, self.headers
        )
    }
}

#[derive(Debug)]
pub struct HttpFile {
    pub requests: Vec<HttpRequest>,
}

impl<'i> From<Pair<'i, Rule>> for HttpFile {
    fn from(pair: Pair<Rule>) -> Self {
        let requests = pair
            .into_inner()
            .filter_map(|pair| match pair.as_rule() {
                Rule::request => Some(pair.into()),
                _ => None,
            })
            .collect();

        Self { requests }
    }
}

impl Display for HttpFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.requests.is_empty() {
            writeln!(f, "No requests found")?;
            return Ok(());
        }
        for (i, req) in self.requests.iter().enumerate() {
            write!(f, "#{i}\n{req}\n")?;
        }
        Ok(())
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
    use super::{parse, HttpFile};
    use reqwest::{Method, Version};

    fn assert_parses(input: &str) -> HttpFile {
        let parsed = parse(input);
        assert!(parsed.is_ok());
        parsed.unwrap()
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
        assert_eq!(file.requests[0].url, "foo.bar");
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
    fn test_headers() {
        let input = r#"
POST test.dev HTTP/1.0
authorization: Bearer xxxx

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].headers.0.len(), 1);
        assert_eq!(
            file.requests[0].headers.0.get("authorization").unwrap(),
            "Bearer xxxx"
        );
    }

    #[test]
    fn test_body() {
        let input = r#"
POST test.dev HTTP/1.0

{ "test": "body" }"#;
        let file = assert_parses(input);
        assert_eq!(file.requests[0].body, "{ \"test\": \"body\" }");
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
POST test.dev?foo=bar&baz=2 HTTP/1.0
authorization: token

"#;
        let file = assert_parses(input);
        assert_eq!(file.requests.len(), 1);
        assert_eq!(file.requests[0].query.len(), 2);
        assert_eq!(file.requests[0].query.get("foo").unwrap(), "bar");
        assert_eq!(file.requests[0].query.get("baz").unwrap(), "2");
    }
}
