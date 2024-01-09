// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ax_event_service;

use ax_event_service::{
    ax_types::{
        app_id,
        service::QueryRequest,
        service::{EventResponse, QueryResponse},
        Payload,
    },
    AxThreadParams, BindTo, EventServiceBlockingRef,
};
use ax_types::service::Order;
use futures::{future::FutureExt, stream::StreamExt};
use tauri::State;

fn main() {
    let is_alive = std::sync::atomic::AtomicBool::new(true);
    let (ax_service_lock, ax_thread, ax_init) =
        ax_event_service::init(move || is_alive.load(std::sync::atomic::Ordering::Relaxed));

    tauri::Builder::default()
        .setup(move |app| {
            // Supply storage_dir path and bind_to to actyx
            ax_init(AxThreadParams {
                bind_to: BindTo::random()?,
                storage_dir: tauri::api::path::app_local_data_dir(&*app.config())
                    .ok_or(anyhow::anyhow!("app_local_data_dir fails"))?
                    .join("actyx"),
            })?;

            Ok(())
        })
        .manage(ax_service_lock)
        .invoke_handler(tauri::generate_handler![query_all_events])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    if let Err(x) = ax_thread.join() {
        eprintln!("{:?}", x);
    }
}

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn query_all_events(
    ax_service_lock: State<ax_event_service::EventServiceLock>,
) -> Result<Vec<EventResponse<Payload>>, String> {
    EventServiceBlockingRef::try_from(&*ax_service_lock)
        .map_err(|e| e.to_string())?
        .exec(|service| {
            async move {
                Ok(service
                    .query(
                        app_id!("app.example.com"),
                        QueryRequest {
                            query: "FROM allEvents".into(),
                            lower_bound: None,
                            upper_bound: None,
                            order: Order::Asc,
                        },
                    )
                    .await
                    .map_err(|e| format!("streaming error {:?}", e))?
                    .filter_map(move |e| async {
                        if let QueryResponse::Event(x) = e {
                            Some(x)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .await)
            }
            .boxed()
        })
}
