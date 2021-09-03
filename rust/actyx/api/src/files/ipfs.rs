use actyx_sdk::AppId;
use anyhow::{Context, Result};
use http::header::CONTENT_DISPOSITION;
use libipld::cid::Cid;
use percent_encoding::percent_decode_str;
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

use crate::{
    ans::ActyxNamingService,
    rejections::ApiError,
    util::filters::{authenticate_optional, header_or_query_token_opt},
    NodeInfo,
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

pub fn content_type_from_ext(name: &str) -> Option<HeaderValue> {
    let ext = Path::new(name).extension()?.to_str()?;
    let mime = mime_guess::from_ext(ext).first_raw()?;
    debug!("detected mime type {} from extension ({})", mime, ext);
    HeaderValue::from_str(mime).ok()
}

pub fn content_type_from_content(chunk: &[u8]) -> Option<HeaderValue> {
    let mime = tree_magic::from_u8(chunk);
    debug!("detected mime type {} from content", mime);
    HeaderValue::from_str(&mime).ok()
}

#[tracing::instrument(level = "debug", skip(store))]
pub(crate) async fn get_file(store: BanyanStore, cid: Cid, name: &str) -> Result<Response<Body>, anyhow::Error> {
    let content_header_from_ext = content_type_from_ext(name);
    let tmp = store.ipfs().create_temp_pin()?;
    store.ipfs().temp_pin(&tmp, &cid)?;

    store.ipfs().sync(&cid, store.ipfs().peers()).await?;
    let mut buf = vec![];
    store.cat(&cid, &mut buf)?;

    // extension takes precedence over content
    let maybe_content_type = content_header_from_ext.or_else(|| {
        tracing::span!(tracing::Level::DEBUG, "Detecting content-type", %cid, size=buf.len());
        // This is fairly expensive, so only look at the first kb
        content_type_from_content(&buf[0..buf.len().min(1024)])
    });
    let mut resp = Response::new(Body::from(buf));
    if name.len() > 0 {
        resp.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&*format!(r#"inline;filename="{}""#, name))?,
        );
    }
    if let Some(ct) = maybe_content_type {
        resp.headers_mut().insert(CONTENT_TYPE, ct);
    }
    Ok(resp)
}

pub(crate) fn extract_name_or_cid_from_host(
    ans: &ActyxNamingService,
    input: &str,
    token_valid: bool,
) -> anyhow::Result<Cid> {
    let (sub, _) = input
        .split_once(".actyx.localhost")
        .context("No subdomain given before .actyx.localhost")?;

    if let Some(record) = ans.get(sub) {
        if !record.public && !token_valid {
            Err(ApiError::MissingAuthorizationHeader.into())
        } else {
            Ok(record.cid)
        }
    } else if !token_valid {
        // Providing cids always needs an auth header
        Err(ApiError::MissingAuthorizationHeader.into())
    } else {
        sub.parse().context("Not a valid multihash")
    }
}

pub(crate) fn extract_query_from_host(
    node_info: NodeInfo,
    ans: ActyxNamingService,
) -> impl Filter<Extract = (IpfsQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path::full())
        .and(warp::host::optional())
        .and(authenticate_optional(node_info, header_or_query_token_opt()))
        .and_then(
            move |full_path: FullPath, authority: Option<Authority>, app_id: Option<AppId>| {
                let r = if let Some(a) = authority {
                    percent_decode_str(full_path.as_str())
                        .decode_utf8()
                        .map_err(Into::into)
                        .and_then(|decoded| {
                            extract_name_or_cid_from_host(&ans, a.host(), app_id.is_some()).map(|root| {
                                let path = decoded
                                    .split('/')
                                    .filter(|x| !x.is_empty())
                                    .map(|x| x.to_owned())
                                    .collect::<VecDeque<_>>();
                                IpfsQuery { root, path }
                            })
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
    path::tail().and_then(move |path_tail: Tail| {
        let check = || {
            let decoded = percent_decode_str(path_tail.as_str()).decode_utf8()?;
            let mut path = decoded.split('/').filter(|x| !x.is_empty());

            let root_or_name = path.next().context("Empty root path")?;
            let root: Cid = ans
                .get(root_or_name)
                .map(|x| x.cid)
                .context("No ANS record found")
                .or_else(|_| root_or_name.parse())
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
