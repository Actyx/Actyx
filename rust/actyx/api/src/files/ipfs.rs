use actyx_sdk::AppId;
use anyhow::{Context, Result};
use futures::{Stream, StreamExt};
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
    ans::{ActyxName, ActyxNamingService},
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

pub fn content_type_from_ext(name: &str) -> Option<String> {
    let ext = Path::new(name).extension()?.to_str()?;
    let mime = mime_guess::from_ext(ext).first_raw()?;
    debug!("detected mime type {} from extension ({})", mime, ext);
    Some(mime.into())
}

pub fn content_type_from_content(chunk: &[u8]) -> Option<&'static str> {
    let mime = tree_magic_mini::from_u8(chunk);
    debug!("detected mime type {} from content", mime);
    Some(mime)
}

pub async fn get_file(store: BanyanStore, cid: Cid) -> anyhow::Result<impl Stream<Item = anyhow::Result<Vec<u8>>>> {
    let mut tmp = store.ipfs().create_temp_pin()?;
    store.ipfs().temp_pin(&mut tmp, &cid)?;

    Ok(store.cat(cid, false))
}

pub(crate) async fn get_file_raw(store: BanyanStore, cid: Cid, name: &str) -> anyhow::Result<Response<Body>> {
    let s = get_file(store, cid).await?;
    let mut response = if let Some(ct) = content_type_from_ext(name) {
        let mut r = Response::new(Body::wrap_stream(s));
        r.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_str(&ct)?);
        r
    } else {
        let mut s = Box::pin(s.peekable());
        let buf = s
            .as_mut()
            .peek()
            .await
            .context("empty stream")?
            .as_ref()
            .map_err(|e| anyhow::anyhow!("{:#}", e))?;
        tracing::debug!(%cid, %name, size=buf.len(), "Detecting content-type from content");

        let ct = content_type_from_content(&buf[..buf.len().min(1024)]);
        let mut r = Response::new(Body::wrap_stream(s));
        if let Some(ct) = ct {
            r.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_str(ct)?);
        }
        r
    };

    if !name.is_empty() {
        response.headers_mut().insert(
            CONTENT_DISPOSITION,
            HeaderValue::from_str(&*format!(r#"inline;filename="{}""#, name))?,
        );
    }
    Ok(response)
}

pub(crate) fn extract_name_or_cid_from_host(
    ans: &ActyxNamingService,
    input: &str,
    token_valid: bool,
) -> anyhow::Result<(Cid, Option<ActyxName>)> {
    let (sub, _) = input
        .split_once(".actyx.localhost")
        .context("No subdomain given before .actyx.localhost")?;

    if let Some(record) = ans.get(sub) {
        if !record.public && !token_valid {
            Err(ApiError::MissingAuthorizationHeader.into())
        } else {
            Ok((record.cid, Some(sub.into())))
        }
    } else if !token_valid {
        // Providing cids always needs an auth header
        Err(ApiError::MissingAuthorizationHeader.into())
    } else {
        let cid = sub.parse().context("Not a valid multihash")?;
        Ok((cid, None))
    }
}

pub(crate) fn extract_query_from_host(
    node_info: NodeInfo,
    ans: ActyxNamingService,
) -> impl Filter<Extract = ((IpfsQuery, Option<ActyxName>),), Error = Rejection> + Clone {
    warp::get()
        .and(path::full())
        .and(warp::host::optional())
        .and(authenticate_optional(node_info, header_or_query_token_opt()))
        .and_then(
            move |full_path: FullPath, authority: Option<Authority>, app_id: Option<AppId>| {
                let r = match authority {
                    Some(a) if a.host().contains(".actyx.localhost") => percent_decode_str(full_path.as_str())
                        .decode_utf8()
                        .map_err(Into::into)
                        .and_then(|decoded| {
                            extract_name_or_cid_from_host(&ans, a.host(), app_id.is_some()).map(|(root, maybe_name)| {
                                let path = decoded
                                    .split('/')
                                    .filter(|x| !x.is_empty())
                                    .map(|x| x.to_owned())
                                    .collect::<VecDeque<_>>();
                                (IpfsQuery { root, path }, maybe_name)
                            })
                        })
                        .map_err(|e: anyhow::Error| {
                            warp::reject::custom(ApiError::BadRequest {
                                cause: format!("{}", e),
                            })
                        }),
                    _ => Err(warp::reject::not_found()),
                };
                async move { r }
            },
        )
}

pub(crate) fn extract_query_from_path(
    ans: ActyxNamingService,
) -> impl Filter<Extract = ((IpfsQuery, Option<ActyxName>),), Error = Rejection> + Clone {
    path::tail().and_then(move |path_tail: Tail| {
        let check = || {
            let decoded = percent_decode_str(path_tail.as_str()).decode_utf8()?;
            let mut path = decoded.split('/').filter(|x| !x.is_empty());

            let root_or_name = path.next().context("Empty root path")?;
            let (root, maybe_name) = if let Some(r) = ans.get(root_or_name) {
                (r.cid, Some(root_or_name.into()))
            } else {
                let cid: Cid = root_or_name
                    .parse()
                    .context("Provided root is neither a name nor a CID")?;
                (cid, None)
            };
            Ok((
                IpfsQuery {
                    root,
                    path: path.map(|x| x.to_owned()).collect(),
                },
                maybe_name,
            ))
        };

        let r = check().map_err(|e: anyhow::Error| {
            warp::reject::custom(ApiError::BadRequest {
                cause: format!("{}", e),
            })
        });
        async move { r }
    })
}
