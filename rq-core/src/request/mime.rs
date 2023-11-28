use bytes::Bytes;
use mime::{Mime, Name};
use reqwest::{header::CONTENT_TYPE, Response};

use super::decode::decode_with_encoding;

#[derive(Debug, Clone)]
pub struct BytePayload {
    pub extension: Option<String>,
    pub bytes: Bytes,
}

#[derive(Debug, Clone)]
pub struct TextPayload {
    pub extension: Option<String>,
    pub charset: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Payload {
    Bytes(BytePayload),
    Text(TextPayload),
}

impl Payload {
    pub async fn of_response(response: Response) -> Payload {
        let mime = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());

        match mime {
            Some(mime) => {
                let extension = mime.subtype();

                match (mime.type_(), extension) {
                    (mime::TEXT, _) | (_, mime::JSON) => {
                        let charset = mime
                            .get_param("charset")
                            .map_or("utf-8".into(), |charset| charset.to_string());
                        let (text, encoding) =
                            decode_with_encoding(&response.bytes().await.unwrap(), &charset);
                        Payload::Text(TextPayload {
                            charset: encoding.name().to_owned(),
                            text,
                            extension: parse_extension(extension),
                        })
                    }
                    (_, extension) => Payload::Bytes(BytePayload {
                        extension: parse_extension(extension),
                        bytes: response.bytes().await.unwrap(),
                    }),
                }
            }
            None => Payload::Bytes(BytePayload {
                extension: None,
                bytes: response.bytes().await.unwrap(),
            }),
        }
    }
}

fn parse_extension(name: Name) -> Option<String> {
    match name {
        mime::PDF => Some("pdf"),
        mime::HTML => Some("html"),
        mime::BMP => Some("bmp"),
        mime::CSS => Some("css"),
        mime::CSV => Some("csv"),
        mime::GIF => Some("gif"),
        mime::JAVASCRIPT => Some("js"),
        mime::JPEG => Some("jpg"),
        mime::JSON => Some("json"),
        mime::MP4 => Some("mp4"),
        mime::MPEG => Some("mpeg"),
        mime::PNG => Some("png"),
        mime::SVG => Some("svg"),
        mime::XML => Some("xml"),
        _ => None,
    }
    .map(str::to_string)
}
