use std::collections::VecDeque;

use actyx_sdk::AppId;
use anyhow::Context;
use bytes::BufMut;
use futures::prelude::*;
use libipld::cid::Cid;
use swarm::{BanyanStore, Block, BufferingTreeBuilder, TreeOptions};
use warp::{path, Buf, Filter, Rejection, Reply};

use crate::{
    ans::{ActyxNamingService, PersistenceLevel},
    util::filters::{authenticate, header_token},
    NodeInfo,
};

use self::ipfs::{extract_query_from_host, extract_query_from_path, handle_query, IpfsQuery};

mod ipfs;

//pub fn files_route(
//    store: BanyanStore,
//    node_info: NodeInfo,
//) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
//    let add = add(store.clone());
//    let ans = ActyxNamingService::new(store.clone());
//    naming_route(ans.clone(), node_info)
//        .or(extract_query_from_host(ans)
//            .and_then(move |query| handle_query(store.clone(), query).map_err(crate::util::reject)))
//        .or(add)
//}

/// Serve GET requests for the server's root, interpreting the full path as a directory query.
/// GET http://:id.actyx.localhost:<port>/query/into/the/directory
/// where :id is either an (ANS) name or a CIDv1 (checked in that order). If the path is empty, and
/// the :id resolves to a directory with multiple files, `index.html` is appended to the query.
pub fn root_serve(store: BanyanStore) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    extract_query_from_host(ActyxNamingService::new(store.clone()))
        .and_then(move |query| query_root_or_index(store.clone(), query).map_err(crate::util::reject))
}

async fn query_root_or_index(store: BanyanStore, query: IpfsQuery) -> anyhow::Result<hyper::Response<hyper::Body>> {
    match handle_query(store.clone(), query.clone()).await {
        Err(e) if query.path.is_empty() => {
            handle_query(
                store,
                IpfsQuery {
                    root: query.root,
                    path: {
                        let mut vec = VecDeque::with_capacity(1);
                        vec.push_front("index.html".to_string());
                        vec
                    },
                },
            )
            .map(|x| {
                x.context("Resolved neither with empty path nor with `index.html`")
                    .with_context(|| format!("{:#}", e))
            })
            .await
        }
        o => o,
    }
}
// api/v2/files
//   POST: add files
// api/v2/files/:id
// :id can either be a name or a cid
//   GET (this is also reachable w/o auth from the root
//   DELETE: delete name or cid
//   PUT: content is name; update `name` pointing to `id`
pub fn route(
    store: BanyanStore,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let auth = authorize(node_info);
    auth.map(|_| ())
        .untuple_one()
        .and(add(store.clone()))
        .or(get(store.clone()))
        .or(delete_name_or_cid(store.clone()))
        .or(update_name(store))
}

fn get(store: BanyanStore) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::get().and(
        extract_query_from_path(ActyxNamingService::new(store.clone()))
            .and_then(move |query: IpfsQuery| query_root_or_index(store.clone(), query).map_err(crate::util::reject)),
    )
}

fn delete_name_or_cid(store: BanyanStore) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let ans = ActyxNamingService::new(store);
    warp::delete()
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(move |cid_or_name: String| {
            let ans = ans.clone();
            async move {
                if let Some(x) = ans.remove(&*cid_or_name).await.map_err(crate::util::reject)? {
                    Ok(x.cid.to_string())
                } else {
                    // TODO: Remove cid? Removing a named pin will unalias the block, so GC will
                    // eventually remove it .. So this depends a bit on which semantics we want the
                    // `delete` call to have on CIDs
                    Err(warp::reject::not_found())
                }
            }
        })
}

fn update_name(store: BanyanStore) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let ans = ActyxNamingService::new(store);
    warp::put()
        .and(path::param())
        .and(warp::body::json())
        .and_then(move |name: String, maybe_cid: String| {
            let ans = ans.clone();
            async move {
                tracing::debug!(%name, ?maybe_cid, "ANS POST");
                let cid: Cid = maybe_cid.parse()?;
                ans.set(name, cid, PersistenceLevel::Prefetch).await?;
                Ok(warp::reply())
            }
            .map_err(crate::util::reject)
        })
}

fn add(store: BanyanStore) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        .and(warp::path::end())
        // TODO: add auth
        .and(warp::multipart::form().max_length(128 << 20))
        .and_then(move |mut form: warp::multipart::FormData| {
            let store = store.clone();
            async move {
                let mut opts = TreeOptions::default();
                opts.wrap_with_directory();
                let mut builder = BufferingTreeBuilder::new(opts);
                let tmp = store.ipfs().create_temp_pin()?;
                while let Some(part) = form.try_next().await? {
                    tracing::debug!("part {:?}", part);
                    let name = part.filename().context("No filename provided")?.to_string();

                    // TODO: use a named pin and store it somewhere?
                    let data = part
                        .stream()
                        .try_fold(Vec::new(), |mut vec, data| {
                            vec.put(data);
                            async move { Ok(vec) }
                        })
                        .await?;
                    let (cid, bytes_written) = store.add(&tmp, data.reader())?;
                    tracing::debug!(%cid, %bytes_written, %name, "Added");
                    builder.put_link(&*name, cid, bytes_written as u64)?;
                }
                let mut root = None;
                for node in builder.build() {
                    let node = node.context("Constructing a directory node")?;
                    store.ipfs().temp_pin(&tmp, &node.cid)?;
                    root = Some(node.cid);
                    let block = Block::new_unchecked(node.cid, node.block.to_vec());
                    store.ipfs().insert(&block)?;
                }
                Ok(root.context("No files provided")?.to_string())
            }
            .map_err(|e| {
                tracing::error!("Error adding files {:#}", e);
                crate::util::reject(e)
            })
        })
}

fn authorize(node_info: NodeInfo) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    authenticate(node_info, header_token())
}
