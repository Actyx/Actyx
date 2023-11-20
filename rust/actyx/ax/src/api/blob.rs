use crate::{
    api::{
        filters::{authenticate, header_or_query_token},
        rejections::ApiError,
        NodeInfo,
    },
    balanced_or,
    swarm::blob_store::{BlobStore, BlobTooLarge},
};
use actyx_sdk::AppId;
use bytes::Bytes;
use http::StatusCode;
use std::{borrow::Cow, convert::TryFrom};
use warp::{
    body::bytes,
    delete, get,
    header::{header, optional},
    http::Response,
    path::{self, Tail},
    post, put,
    reject::{self},
    Filter, Rejection, Reply,
};

fn content_length(limit: usize) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    optional::<usize>("Content-Length")
        .and_then(move |cl| async move {
            let error = if let Some(cl) = cl {
                if cl <= limit {
                    return Ok(());
                } else {
                    ApiError::TooLarge { size: cl, limit }
                }
            } else {
                ApiError::LengthUnknown { limit }
            };
            tracing::warn!("rejecting upload: {}", error);
            Err(reject::custom(error))
        })
        .untuple_one()
}

pub(crate) fn routes(
    store: BlobStore,
    node_info: NodeInfo,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let f = authenticate(node_info, header_or_query_token())
        .and(path::param().and_then(|app: String| async move {
            AppId::try_from(&*app).map_err(|e| reject::custom(ApiError::BadRequest { cause: e.to_string() }))
        }))
        .and(path::tail())
        .and(warp::any().map(move || store.clone()));
    balanced_or!(
        get().and(f.clone()).and(header("Accept")).and_then(handle_get),
        delete().and(f.clone()).and_then(handle_delete),
        put()
            .and(f.clone())
            .and(content_length(10485760))
            .and(bytes())
            .and(header("Content-Type"))
            .and_then(handle_put),
        post().and(f).and_then(handle_post),
    )
}

async fn handle_get(
    app_id: AppId,
    target: AppId,
    tail: Tail,
    store: BlobStore,
    accept: String,
) -> Result<impl Reply, Rejection> {
    let app = if target.as_str() == "-" { app_id } else { target };
    let path = tail.as_str().to_owned();
    match store.blob_get(app.clone(), path) {
        Ok(Some((data, mime))) => {
            if accept.contains(&*mime) || accept.contains(mime_wild(&mime).as_ref()) || accept.contains("*/*") {
                Ok(Response::builder().header("Content-Type", mime).body(data))
            } else {
                Err(reject::custom(ApiError::NotAcceptable {
                    supported: mime,
                    requested: accept,
                }))
            }
        }
        Ok(None) => Ok(Response::builder().status(StatusCode::NO_CONTENT).body(vec![])),
        Err(err) => {
            tracing::error!("error while getting blob {}/{}: {}", app, tail.as_str(), err);
            Err(reject::custom(ApiError::Internal))
        }
    }
}

fn mime_wild(mime: &str) -> Cow<str> {
    if let Some(idx) = mime.find('/') {
        Cow::Owned(format!("{}/*", &mime[..idx]))
    } else {
        Cow::Borrowed(mime)
    }
}

async fn handle_delete(app_id: AppId, target: AppId, tail: Tail, store: BlobStore) -> Result<impl Reply, Rejection> {
    if target.as_str() != "-" {
        return Err(reject::custom(ApiError::BadRequest {
            cause: format!("cannot delete blob for specific appId {}", target),
        }));
    }
    let path = tail.as_str().to_owned();
    match store.blob_del(app_id.clone(), path) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            tracing::error!("error while deleting blob {}/{}: {}", app_id, tail.as_str(), err);
            Err(reject::custom(ApiError::Internal))
        }
    }
}

async fn handle_put(
    app_id: AppId,
    target: AppId,
    tail: Tail,
    store: BlobStore,
    bytes: Bytes,
    mime_type: String,
) -> Result<impl Reply, Rejection> {
    if target.as_str() != "-" {
        return Err(reject::custom(ApiError::BadRequest {
            cause: format!("cannot put blob for specific appId {}", target),
        }));
    }
    let path = tail.as_str().to_owned();
    match store.blob_put(app_id.clone(), path, mime_type, bytes.as_ref()) {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(err) => {
            if let Some(err) = err.downcast_ref::<BlobTooLarge>() {
                tracing::warn!("error while putting blob {}/{}: {}", app_id, tail.as_str(), err);
                Err(reject::custom(ApiError::TooLarge {
                    size: err.size,
                    limit: err.limit,
                }))
            } else {
                tracing::error!("error while putting blob {}/{}: {}", app_id, tail.as_str(), err);
                Err(reject::custom(ApiError::Internal))
            }
        }
    }
}

async fn handle_post(_app_id: AppId, _target: AppId, _tail: Tail, _store: BlobStore) -> Result<impl Reply, Rejection> {
    Ok(StatusCode::NO_CONTENT)
}
