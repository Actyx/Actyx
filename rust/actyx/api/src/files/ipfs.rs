use anyhow::{Context, Result};
use libipld::cid::Cid;
use std::{collections::VecDeque, path::Path, str::FromStr};
use swarm::BanyanStore;
use tracing::*;
use warp::{
    host::Authority,
    http::header::{HeaderValue, CONTENT_TYPE},
    hyper::{Body, Response},
    path::{self, FullPath, Tail},
    Filter, Rejection,
};

use crate::{ans::ActyxNamingService, rejections::ApiError};

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

fn content_type_from_content(chunk: &[u8]) -> Option<HeaderValue> {
    let mime = tree_magic::from_u8(chunk);
    debug!("detected mime type {} from content", mime);
    HeaderValue::from_str(&mime).ok()
}

#[tracing::instrument(level = "debug", skip(store))]
pub(crate) async fn handle_query(store: BanyanStore, query: IpfsQuery) -> Result<Response<Body>, anyhow::Error> {
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
    // extension takes precedence over content
    if let Some(ct) = content_header_from_ext.or_else(|| {
        tracing::span!(tracing::Level::DEBUG, "Detecting content-type", %cid, size=buf.len());
        // This is fairly expensive, so only look at the first kb
        content_type_from_content(&buf[0..buf.len().min(1024)])
    }) {
        let mut resp = Response::new(Body::from(buf));
        resp.headers_mut().insert(CONTENT_TYPE, ct);
        Ok(resp)
    } else {
        Ok(Response::new(Body::from(buf)))
    }
}

pub(crate) fn extract_name_or_cid_from_host(ans: &ActyxNamingService, input: &str) -> anyhow::Result<Cid> {
    let (sub, _) = input
        .split_once(".actyx.localhost")
        .context("No subdomain given before .actyx.localhost")?;
    sub.parse()
        .or_else(|_| ans.get(sub).context("No ANS Record found").map(|x| x.cid))
}

pub(crate) fn extract_query_from_host(
    ans: ActyxNamingService,
) -> impl Filter<Extract = (IpfsQuery,), Error = Rejection> + Clone {
    warp::get().and(path::full()).and(warp::host::optional()).and_then(
        move |full_path: FullPath, authority: Option<Authority>| {
            let r = if let Some(a) = authority {
                extract_name_or_cid_from_host(&ans, a.host())
                    .context("Sub domain must be a valid multihash")
                    .map(|root| {
                        let path = full_path
                            .as_str()
                            .split('/')
                            .filter(|x| !x.is_empty())
                            .map(|x| x.to_owned())
                            .collect::<VecDeque<_>>();
                        IpfsQuery { root, path }
                    })
                    .map_err(|e: anyhow::Error| {
                        warp::reject::custom(ApiError::BadRequest {
                            cause: format!("{}", e),
                        })
                    })
            } else {
                Err(warp::reject::not_found())
            };
            async move { r }
        },
    )
}

pub(crate) fn extract_query_from_path(
    ans: ActyxNamingService,
) -> impl Filter<Extract = (IpfsQuery,), Error = Rejection> + Clone {
    path::tail().and_then(move |full_path: Tail| {
        let check = || {
            let mut path = full_path.as_str().split('/').filter(|x| !x.is_empty());

            let maybe_root = path.next().context("Empty root path")?;
            let root: Cid = ans
                .get(maybe_root)
                .map(|x| x.cid)
                .context("No ANS record found")
                .or_else(|_| maybe_root.parse())
                .context("Provided root is neither a name nor a CID")?;
            Ok(IpfsQuery {
                root,
                path: path.map(|x| x.to_owned()).collect(),
            })
        };

        let r = check().map_err(|e: anyhow::Error| {
            warp::reject::custom(ApiError::BadRequest {
                cause: format!("{}", e),
            })
        });
        async move { r }
    })
}
