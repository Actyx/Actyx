use futures::task::Context;
use futures::task::Poll;
use futures::Stream;
use lazy_static::*;
use std::pin::Pin;

#[derive(Debug)]
pub struct ChunkedResponse<S> {
    resp: S,
    last: Vec<u8>,
}
impl<S: Unpin> Unpin for ChunkedResponse<S> {}

impl<S, T> ChunkedResponse<S>
where
    S: Stream<Item = Result<Vec<u8>, T>>,
{
    pub fn new(resp: S) -> Self {
        ChunkedResponse { resp, last: Vec::new() }
    }
}

impl<S, T> Stream for ChunkedResponse<S>
where
    S: Unpin,
    S: Stream<Item = Result<Vec<u8>, T>>,
    T: std::fmt::Debug,
{
    type Item = Result<Vec<String>, T>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let ChunkedResponse { resp, last } = self.get_mut();
        let resp_stream = Pin::new(resp);
        poll_resp(resp_stream, cx, last)
    }
}

#[allow(clippy::type_complexity)]
fn poll_resp<S, T>(resp: Pin<&mut S>, cx: &mut Context<'_>, last: &mut Vec<u8>) -> Poll<Option<Result<Vec<String>, T>>>
where
    S: Stream,
    S: Stream<Item = Result<Vec<u8>, T>>,
    T: std::fmt::Debug,
{
    match resp.poll_next(cx) {
        Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(get_events(item.as_ref(), last)))),
        Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
        Poll::Ready(None) => Poll::Ready(None),
        Poll::Pending => Poll::Pending,
    }
}

// Fold operator
// - untranslated remainder of the concatenated string is left in last (the accumulator)
// - translated parts are emitted as Vec<String>
// invariant: last does not contain a newline (otherwise its contents will be emitted with a delay)
fn get_events(chunk: &[u8], last: &mut Vec<u8>) -> Vec<String> {
    lazy_static! {
        static ref RE: regex::Regex = regex::Regex::new(r"\r?\n").unwrap();
    }
    if chunk.is_empty() {
        // if 0 length chunk, not sure if this is even possible
        return vec![];
    }

    let newline: u8 = b'\n';

    let l = chunk.len();
    let mut split_at = 0;
    for pos in 1..=l {
        if chunk[l - pos] == newline {
            split_at = l - pos + 1; // range 1..pos
            break;
        }
    }

    // if we found a newline, split_at > 0 otherwise == 0 and everything goes into remainder
    let (first, remainder) = chunk.split_at(split_at);
    last.extend_from_slice(first);

    let events = if split_at > 0 {
        let lines = String::from_utf8_lossy(&last);
        let events = RE
            .split(&lines)
            .filter(|l| !l.is_empty())
            .map(|r| r.to_string())
            .collect();
        last.clear();
        events
    } else {
        Vec::new()
    };

    last.extend_from_slice(remainder);

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{stream, StreamExt};
    async fn extract_events(chunks: Vec<&str>) -> Vec<String> {
        let chunks_string: Vec<Result<String, _>> = chunks
            .into_iter()
            .map(|x| Ok::<String, Box<dyn std::error::Error + Send + Sync>>(x.to_owned()))
            .collect();
        let mut chunkedresponse = Box::pin(ChunkedResponse::new(
            hyper::Body::wrap_stream(stream::iter(chunks_string)).map(|s| s.map(|v| v.to_vec())),
        ));
        let collect = async {
            let mut ret = Vec::new();
            while let Some(value) = chunkedresponse.next().await {
                ret.push(value.unwrap());
            }
            ret
        };
        let result: Vec<Vec<String>> = collect.await;
        result.into_iter().flatten().collect::<Vec<String>>()
    }

    fn string_to_static_str(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    #[tokio::test]
    async fn get_events() {
        let initial_chunk = r#"{"stream":{"source" :"#;
        let final_chunk =
            r#""oQ","semantics":"com","name":"t"},"timestamp":1,"offset":1,"payload":{"bar":1},"lamport":null}"#;
        let complete_chunk = string_to_static_str(format!("{}{}", initial_chunk, final_chunk));
        let complete_chunk_nl = string_to_static_str(format!("{}\r\n", complete_chunk));
        let final_chunk_nl = string_to_static_str(format!("{}\r\n", final_chunk));
        let keepalive = "\n\r\n";

        let input_stream: Vec<&str> = vec![complete_chunk_nl, initial_chunk, final_chunk_nl, keepalive];
        let result = extract_events(input_stream).await;
        let expected: Vec<&str> = vec![complete_chunk, complete_chunk];
        assert_eq!(result, expected, "One of each");

        // Only keepalive:
        let input_stream: Vec<&str> = vec![keepalive, keepalive, keepalive, keepalive];
        let expected: Vec<&str> = Vec::new();
        assert_eq!(extract_events(input_stream).await, expected, "Only keepalive");

        // Way too many keepalives:

        let input_stream: Vec<&str> = vec![complete_chunk_nl, keepalive, initial_chunk, final_chunk_nl, keepalive];
        let result = extract_events(input_stream).await;
        let expected: Vec<&str> = vec![complete_chunk, complete_chunk];
        assert_eq!(result, expected, "One of each");
    }

    #[tokio::test]
    async fn get_partially_invalid_events() {
        let sparkle_heart: Vec<u8> = vec![240, 159, 146, 150, 10];
        let sparkle_1: Vec<u8> = vec![240, 159, 146];
        let sparkle_2: Vec<u8> = vec![150, 10];

        let mut accumulated_string: Vec<u8> = Vec::new();
        let result = super::get_events(&sparkle_heart, &mut accumulated_string);

        let sparkle_heart_string = String::from_utf8(sparkle_heart).expect("Found invalid UTF-8");
        let expected = vec![sparkle_heart_string.trim_end()];
        assert_eq!(expected, result);

        let mut accumulated_string: Vec<u8> = Vec::new();

        let result = super::get_events(&sparkle_1, &mut accumulated_string);
        assert_eq!(sparkle_1, accumulated_string);
        assert_eq!(Vec::<String>::new(), result);

        let result = super::get_events(&sparkle_2, &mut accumulated_string);
        assert_eq!(expected, result);
    }

    #[allow(clippy::char_lit_as_u8)]
    #[tokio::test]
    async fn split_at_the_right_boundary() {
        let chunk: Vec<u8> = vec!['R' as u8, 'U' as u8, 'S' as u8];

        let mut accumulated_string: Vec<u8> = Vec::new();
        let result = super::get_events(&chunk, &mut accumulated_string);

        assert_eq!(Vec::<String>::new(), result);

        let chunk: Vec<u8> = vec![
            'T' as u8, '\n' as u8, 'I' as u8, 'S' as u8, 'N' as u8, 'O' as u8, 'T' as u8,
        ];

        let result = super::get_events(&chunk, &mut accumulated_string);
        assert_eq!(vec!["RUST"], result);

        let chunk: Vec<u8> = vec![
            '\n' as u8, 'P' as u8, 'Y' as u8, 'T' as u8, 'H' as u8, 'O' as u8, 'N' as u8, '\n' as u8,
        ];

        let result = super::get_events(&chunk, &mut accumulated_string);
        assert_eq!(vec!["ISNOT", "PYTHON"], result);
        assert_eq!(Vec::<u8>::new(), accumulated_string);
    }

    #[allow(clippy::char_lit_as_u8)]
    #[tokio::test]
    async fn split_for_newline_at_the_beginning() {
        let chunk: Vec<u8> = vec!['R' as u8, 'U' as u8, 'S' as u8, 'T' as u8];

        let mut accumulated_string: Vec<u8> = Vec::new();
        let result = super::get_events(&chunk, &mut accumulated_string);

        assert_eq!(Vec::<String>::new(), result);

        let chunk: Vec<u8> = vec!['\n' as u8, 'G' as u8, 'O' as u8, 'O' as u8, 'D' as u8];

        let result = super::get_events(&chunk, &mut accumulated_string);
        assert_eq!(vec!["RUST"], result);

        let chunk: Vec<u8> = vec!['\n' as u8];

        let result = super::get_events(&chunk, &mut accumulated_string);
        assert_eq!(vec!["GOOD"], result);
        assert_eq!(Vec::<u8>::new(), accumulated_string);
    }
}
