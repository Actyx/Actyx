use std::{fmt::Write, path::Path, str::FromStr, time::Duration};

use actyx_sdk::{
    app_id,
    service::{DirectoryChild, FilesGetResponse, PrefetchRequest},
    tags, AppId, Payload,
};
use anyhow::Context;
use bytes::{BufMut, Bytes};
use futures::prelude::*;
use http::{header::CACHE_CONTROL, Uri};
use libipld::cid::Cid;
use serde::Serialize;
use swarm::{BanyanStore, Block, BufferingTreeBuilder, TreeOptions};
use warp::{
    path::{self, FullPath},
    Buf, Filter, Rejection, Reply,
};

use self::ipfs::{extract_query_from_host, extract_query_from_path, IpfsQuery};
use crate::{
    ans::{ActyxName, ActyxNamingService, PersistenceLevel},
    balanced_or,
    rejections::ApiError,
    util::filters::{authenticate, header_or_query_token},
    NodeInfo,
};
pub(crate) use pinner::FilePinner;

mod ipfs;
mod pinner;

/// Serve GET requests for the server's root, interpreting the full path as a directory query.
/// GET http://:id.actyx.localhost:<port>/query/into/the/directory
/// where :id is either an (ANS) name or a CIDv1 (checked in that order). If the path is empty, and
/// the :id resolves to a directory with multiple files, `index.html` is appended to the query.
pub fn root_serve(
    store: BanyanStore,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::header::optional(http::header::ACCEPT.as_str())
        .and(extract_query_from_host(
            node_info,
            ActyxNamingService::new(store.clone()),
        ))
        .and(warp::path::full())
        .and(query_raw_opt())
        .and_then(
            move |accept_header: Option<String>,
                  (query, maybe_name): (IpfsQuery, Option<ActyxName>),
                  uri_path: FullPath,
                  raw_query: Option<String>| {
                serve_unixfs_node(
                    store.clone(),
                    query,
                    uri_path,
                    raw_query,
                    accept_header,
                    true,
                    maybe_name,
                )
                .map_err(crate::util::reject)
            },
        )
}

fn query_raw_opt() -> impl Filter<Extract = (Option<String>,), Error = Rejection> + Clone {
    warp::filters::query::raw()
        .map(Some)
        .recover(|_| async move { Ok(None) })
        .unify()
}

async fn serve_unixfs_node(
    store: BanyanStore,
    query: IpfsQuery,
    uri_path: FullPath,
    raw_query: Option<String>,
    accept_headers: Option<String>,
    auto_serve_index_html: bool,
    ans_name: Option<ActyxName>,
) -> anyhow::Result<impl Reply> {
    let mut response = match store.unixfs_resolve_path(query.root, query.path).await? {
        swarm::FileNode::Directory {
            children,
            name,
            own_cid,
        } => {
            if accept_headers
                .as_deref()
                .map(|x| x.to_lowercase().contains("text/html"))
                .unwrap_or_default()
            {
                if let Some(index_html) = auto_serve_index_html
                    .then(|| children.iter().find(|x| &*x.name == "index.html"))
                    .flatten()
                {
                    ipfs::get_file_raw(store, index_html.cid, &index_html.name).await?
                } else if !uri_path.as_str().ends_with('/') {
                    // Add trailing slash so the links in the directory listings
                    // work as intended.
                    let uri = format!(
                        "{}/{}",
                        uri_path.as_str(),
                        raw_query.map(|q| format!("?{}", q)).unwrap_or_default(),
                    );
                    warp::redirect(Uri::from_str(&uri)?).into_response()
                } else {
                    let body = render_directory_listing(name, own_cid, children, raw_query)?;
                    warp::reply::html(body).into_response()
                }
            } else {
                let r = FilesGetResponse::Directory {
                    name,
                    cid: own_cid,
                    children: children
                        .into_iter()
                        .map(|c| DirectoryChild {
                            cid: c.cid,
                            name: c.name,
                            size: c.size,
                        })
                        .collect(),
                };
                warp::reply::json(&r).into_response()
            }
        }
        swarm::FileNode::File { cid, name } => {
            if accept_headers
                .as_deref()
                .map(|x| x.to_lowercase().contains("application/json"))
                .unwrap_or_default()
            {
                warp::reply::json(&ipfs::get_file_structured(store, cid, &name).await?).into_response()
            } else {
                ipfs::get_file_raw(store, cid, &name).await?
            }
        }
    };
    if ans_name.is_some() {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, "no-cache, no-store, must-revalidate".parse().unwrap());
    }
    Ok(response)
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
    pinner: FilePinner,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    balanced_or!(
        warp::path("prefetch").and(prefetch(pinner, node_info.clone())),
        add(store.clone(), node_info.clone()),
        get(store.clone(), node_info.clone()),
        delete_name_or_cid(store.clone(), node_info.clone()),
        update_name(store, node_info)
    )
}

fn prefetch(
    pinner: FilePinner,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post().and(authorize(node_info)).and(warp::body::json()).and_then(
        move |app_id: AppId, request: PrefetchRequest| {
            pinner
                .update(app_id, request.query)
                .map(|_| Ok(http::StatusCode::NO_CONTENT))
                .map_err(crate::util::reject)
        },
    )
}

// TODO: Make this a bit nicer. Also take the path to `node` into account to provide upwards
// traversal.
fn render_directory_listing(
    name: String,
    cid: Cid,
    children: Vec<swarm::Child>,
    raw_query: Option<String>,
) -> anyhow::Result<String> {
    let mut body = String::new();
    let query = raw_query.map(|q| format!("?{}", q)).unwrap_or_default();

    write!(
        &mut body,
        r#"
<!DOCTYPE html>
<head>
<title>Actyx Files: Directory {}</title>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
</head>
<body>
<table>
  <tr>
    <th>Name</th>
    <th>Size</th>
    <th>Cid</th>
  </tr>
  <tr>
    <td>. ({})</a></td>
    <td></td>
    <td>{}</td>
  </tr>"#,
        name, name, cid
    )?;
    for swarm::Child { cid, name, size } in children {
        write!(
            &mut body,
            r#"
<tr>
  <td><a href='{}{}'>{}</a></td>
  <td>{}</td>
  <td>{}</td>
</tr>"#,
            name, query, name, size, cid
        )?;
    }
    write!(&mut body, "</table></body>")?;

    Ok(body)
}

fn get(store: BanyanStore, node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::get()
        .and(authorize(node_info).map(|_| ()).untuple_one())
        .and(warp::header::optional(http::header::ACCEPT.as_str()))
        .and(extract_query_from_path(ActyxNamingService::new(store.clone())))
        .and(warp::path::full())
        .and(query_raw_opt())
        .and_then(
            move |accept_header: Option<String>,
                  (query, maybe_name): (IpfsQuery, Option<ActyxName>),
                  uri_path: FullPath,
                  raw_query: Option<String>| {
                serve_unixfs_node(
                    store.clone(),
                    query,
                    uri_path,
                    raw_query,
                    accept_header,
                    false,
                    maybe_name,
                )
                .map_err(crate::util::reject)
            },
        )
}

fn delete_name_or_cid(
    store: BanyanStore,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let ans = ActyxNamingService::new(store);
    warp::delete()
        .and(warp::path::param())
        .and(warp::path::end())
        .and(authorize(node_info).map(|_| ()).untuple_one())
        .and_then(move |cid_or_name: String| {
            let ans = ans.clone();
            async move {
                if let Some(x) = ans
                    .remove(cid_or_name)
                    .await
                    .map_err(|_| warp::reject::custom(ApiError::Internal))?
                {
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

fn update_name(
    store: BanyanStore,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let ans = ActyxNamingService::new(store);
    warp::put()
        .and(path::param())
        .and(authorize(node_info).map(|_| ()).untuple_one())
        .and(warp::body::bytes())
        .and_then(move |name: String, maybe_cid: Bytes| {
            let ans = ans.clone();
            async move {
                tracing::debug!(%name, ?maybe_cid, "ANS POST");
                if name.parse::<Cid>().is_ok() {
                    tracing::error!(%name, ?maybe_cid, "Rejecting because name can be interpreted as a CID");
                    anyhow::bail!("Name must not be a CID")
                } else {
                    let cid = Cid::from_str(&*String::from_utf8(maybe_cid.to_vec())?)?;
                    ans.set(name, cid, PersistenceLevel::Prefetch, true).await?;
                    Ok(warp::reply())
                }
            }
            .map_err(crate::util::reject)
        })
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum FileApiEvent {
    FileAdded {
        name: String,
        // Must not be serialized as a cid!
        #[serde(with = "::actyx_util::serde_str")]
        cid: Cid,
        size: u64,
        mime: String,
        app_id: AppId,
    },
    DirectoryAdded {
        name: String,
        // Must not be serialized as a cid!
        #[serde(with = "::actyx_util::serde_str")]
        cid: Cid,
        size: u64,
        app_id: AppId,
    },
}

fn add(store: BanyanStore, node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let auth = authorize(node_info);
    warp::post()
        .and(warp::path::end())
        .and(auth)
        .and(warp::multipart::form().max_length(128 << 20))
        .and_then(move |app_id: AppId, mut form: warp::multipart::FormData| {
            let store = store.clone();
            async move {
                let tmp = store.ipfs().create_temp_pin()?;
                let mut added_files = vec![];
                while let Some(part) = form.try_next().await? {
                    tracing::debug!("part {:?}", part);
                    let name = {
                        let n = part.filename().context("No filename provided")?;
                        n.strip_prefix('/').unwrap_or(n).to_string()
                    };

                    let data = part
                        .stream()
                        .try_fold(Vec::new(), |mut vec, data| {
                            vec.put(data);
                            async move { Ok(vec) }
                        })
                        .await?;
                    let (cid, bytes_written) = store.add(&tmp, data.reader())?;
                    tracing::debug!(%cid, %bytes_written, %name, "Added");
                    added_files.push((name, (cid, bytes_written)));
                }

                let mut output = None;
                if added_files.len() > 1 {
                    let mut opts = TreeOptions::default();
                    opts.wrap_with_directory();

                    let mut builder = BufferingTreeBuilder::new(opts);
                    for (name, (cid, bytes_written)) in &added_files {
                        builder.put_link(name, *cid, *bytes_written as u64)?;
                    }
                    for node in builder.build() {
                        let node = node.context("Constructing a directory node")?;
                        // FIXME: revisit the pinning behaviour of the files api
                        store.ipfs().temp_pin(&tmp, &node.cid)?;
                        let block = Block::new_unchecked(node.cid, node.block.to_vec());
                        store.ipfs().insert(&block)?;

                        output = Some((
                            node.cid,
                            FileApiEvent::DirectoryAdded {
                                name: Path::new(&node.path)
                                    .file_name()
                                    .map(|x| x.to_string_lossy().into())
                                    .unwrap_or_else(|| "/".into()),
                                cid: node.cid,
                                size: node.total_size,
                                app_id: app_id.clone(),
                            },
                        ));
                    }
                } else if let Some((name, (cid, bytes_written))) = added_files.first() {
                    output = Some((
                        *cid,
                        FileApiEvent::FileAdded {
                            mime: mime(name),
                            name: name.into(),
                            cid: *cid,
                            size: *bytes_written as u64,
                            app_id,
                        },
                    ));
                };
                let (root, event) = output.context("No files provided")?;
                store
                    .append(
                        0.into(),
                        app_id!("com.actyx"),
                        vec![(
                            tags!("files", "files:created"),
                            Payload::compact(&event).expect("serialization works"),
                        )],
                    )
                    .await?;

                // Keep the temp pin around for a short time until the [`FilePinner`] picks up the
                // new root.
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    drop(tmp);
                });
                Ok(root.to_string())
            }
            .map_err(|e| {
                tracing::error!("Error adding files {:#}", e);
                crate::util::reject(e)
            })
        })
}

fn authorize(node_info: NodeInfo) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    authenticate(node_info, header_or_query_token())
}

fn mime(name: impl AsRef<Path>) -> String {
    name.as_ref()
        .extension()
        .and_then(|ext| mime_guess::from_ext(&ext.to_string_lossy()).first())
        .unwrap_or(mime_guess::mime::APPLICATION_OCTET_STREAM)
        .to_string()
}
