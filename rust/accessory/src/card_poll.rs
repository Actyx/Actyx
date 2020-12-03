use super::{AccessoryError, CardReader, HotPlugEvent, HotPlugHandler, ReaderId};
use crossbeam::channel::unbounded;
use rusb::UsbContext;
use serde::Serialize;
use std::{collections::BTreeMap, time::Duration};
use tracing::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardScanned {
    uid: Vec<u8>,
    card_type: Option<String>,
    pub reader: ReaderId,
}

pub fn card_poll_loop(tx_out: Option<tokio::sync::mpsc::Sender<CardScanned>>) -> anyhow::Result<()> {
    let rusb_ctx = rusb::Context::new()?;
    let (tx, rx) = unbounded();
    let _hotplug = HotPlugHandler::try_setup(&rusb_ctx, tx)?;

    // TODO: handle >1 baltech readers
    let mut attached_readers: BTreeMap<ReaderId, CardReader> = Default::default();
    for d in rusb_ctx.devices()?.iter() {
        let id: ReaderId = (&d).into();
        trace!("Attached USB device ID {:?}", id);
        let usb_descriptor = d.device_descriptor()?;
        match CardReader::create_from_usb(id.clone(), &usb_descriptor) {
            Ok(card_reader) => {
                info!("Created reader \"{:?}\"", id);
                attached_readers.insert(id, card_reader);
            }
            Err(e) => match e.downcast_ref::<AccessoryError>() {
                Some(AccessoryError::NotABaltechReader) => {}
                _ => error!("Error creating reader from USB device: {}", e),
            },
        }
    }
    loop {
        // Drive rusb event loop
        rusb_ctx.handle_events(Some(Duration::from_millis(20)))?;

        // Handle hotplug events, either from USB or PCSC
        while let Ok(d) = rx.try_recv() {
            match d {
                HotPlugEvent::UsbAttached((id, d)) => {
                    // Some time for USB initialization (e.g. Baltech needs that before being responive)
                    std::thread::sleep(Duration::from_millis(100));
                    match CardReader::create_from_usb(id.clone(), &*d) {
                        Ok(card_reader) => {
                            attached_readers.insert(id, card_reader);
                        }
                        Err(e) => match e.downcast_ref::<AccessoryError>() {
                            Some(AccessoryError::NotABaltechReader) => {}
                            _ => error!("Error creating reader from USB device: {}", e),
                        },
                    }
                }
                HotPlugEvent::PcscAttached((id, ctx)) => {
                    let reader = CardReader::create_from_pcsc(ctx, id.clone());
                    attached_readers.insert(id, reader);
                }
                HotPlugEvent::Detached(id) => {
                    attached_readers.remove(&id);
                }
            }
        }
        // Try to read from all attached readers
        for (id, reader) in attached_readers.iter_mut() {
            trace!("Trying to read from {}", id);
            match reader.try_read_uid(false) {
                Err(e) => match e.downcast_ref::<AccessoryError>() {
                    Some(AccessoryError::NoCardPresent) => {}
                    _ => error!("Error reading card {}", e),
                },

                Ok(uid) => {
                    info!("Reader \"{}\": Card UID {:X?}", id, uid);
                    if let Some(tx) = tx_out.as_ref() {
                        if let Err(e) = tx.blocking_send(CardScanned {
                            uid,
                            reader: id.clone(),
                            card_type: None, // TODO
                        }) {
                            return Err(e.into());
                        }
                    }
                }
            }
        }

        // Sleep to avoid busy checking all the things
        std::thread::sleep(Duration::from_millis(100));
    }
}
