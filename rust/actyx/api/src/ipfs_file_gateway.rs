use std::{collections::VecDeque, path::Path, str::FromStr};

use anyhow::Result;
use futures::prelude::*;
use libipld::cid::Cid;
use swarm::BanyanStore;
use tracing::*;
use warp::{
    filters::BoxedFilter,
    http::header::{HeaderValue, CONTENT_TYPE},
    hyper::{Body, Response},
    path, Filter, Reply,
};

/// an ipfs query contains a root cid and a path into it
#[derive(Debug, Clone)]
pub struct IpfsQuery {
    pub root: Cid,
    pub path: VecDeque<String>,
}

impl FromStr for IpfsQuery {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        let mut path = s.split('/');
        let root = if let Some(root) = path.next() {
            root.parse()?
        } else {
            return Err(anyhow::anyhow!("expected CID"));
        };
        let path = path.filter(|x| !x.is_empty()).map(|x| x.to_owned()).collect();
        Ok(IpfsQuery { root, path })
    }
}

fn content_type_from_ext(query: &IpfsQuery) -> Option<HeaderValue> {
    let filename = query.path.back()?;
    let ext = Path::new(filename).extension()?.to_str()?;
    let mime = mime_guess::from_ext(ext).first_raw()?;
    debug!("detected mime type {} from extension ({})", mime, ext);
    HeaderValue::from_str(mime).ok()
}

fn content_type_from_first_chunk(chunk: &[u8]) -> Option<HeaderValue> {
    let mime = tree_magic::from_u8(chunk);
    debug!("detected mime type {} from content", mime);
    HeaderValue::from_str(&mime).ok()
}

async fn handle_query(store: BanyanStore, query: IpfsQuery) -> Result<Response<Body>, anyhow::Error> {
    let content_header_from_ext = content_type_from_ext(&query);
    let tmp = store.ipfs().create_temp_pin()?;
    store.ipfs().temp_pin(&tmp, &query.root)?;
    let cid = if let Some(cid) = store.traverse(&query.root, query.path).await? {
        cid
    } else {
        return Err(anyhow::anyhow!("file not found"));
    };
    store.ipfs().sync(&cid, store.ipfs().peers()).await?;
    let mut buf = vec![];
    store.cat(&cid, &mut buf)?;
    let content_header_from_content = content_type_from_first_chunk(&buf);
    let mut resp = Response::new(Body::from(buf));
    // extension takes precedence over content
    if let Some(ct) = content_header_from_ext.or(content_header_from_content) {
        resp.headers_mut().insert(CONTENT_TYPE, ct);
    }
    Ok(resp)
}

pub fn route(store: BanyanStore) -> BoxedFilter<(impl Reply,)> {
    path::tail()
        .and_then(|tail: warp::path::Tail| {
            future::ready(
                tail.as_str()
                    .parse::<IpfsQuery>()
                    .map_err(|_| warp::reject::not_found()),
            )
        })
        .and_then(move |query: IpfsQuery| handle_query(store.clone(), query).map_err(crate::util::reject))
        .boxed()
}
