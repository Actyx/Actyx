use crate::ReaderId;
use crossbeam::channel;
use rusb::{Device, DeviceDescriptor, UsbContext};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryInto,
    ffi::CString,
    fmt,
    ops::Deref,
    sync::Arc,
    time::Duration,
};
use tracing::*;

pub struct HotPlugHandler {
    tx: channel::Sender<HotPlugEvent>,
}

pub enum HotPlugEvent {
    UsbAttached((ReaderId, UsbDescriptor)),
    PcscAttached((ReaderId, pcsc::Context)),
    Detached(ReaderId),
}
impl fmt::Debug for HotPlugEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HotPlugEvent::UsbAttached((id, descr)) => f
                .debug_tuple("HotPlugEvent::UsbArrived")
                .field(id)
                .field(descr)
                .finish(),
            HotPlugEvent::PcscAttached((id, _)) => f.debug_tuple("HotPlugEvent::PcscArrived").field(id).finish(),
            HotPlugEvent::Detached(id) => f.debug_tuple("HotPlugEvent::Left").field(id).finish(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct UsbDescriptor(Arc<DeviceDescriptor>);

impl From<DeviceDescriptor> for UsbDescriptor {
    fn from(d: DeviceDescriptor) -> Self {
        Self(Arc::new(d))
    }
}
impl Deref for UsbDescriptor {
    type Target = DeviceDescriptor;
    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}

impl Clone for HotPlugHandler {
    fn clone(&self) -> Self {
        Self { tx: self.tx.clone() }
    }
}
impl<T: UsbContext> rusb::Hotplug<T> for HotPlugHandler {
    fn device_arrived(&mut self, device: Device<T>) {
        let id = (&device).into();
        match device.device_descriptor() {
            Ok(descr) => self.tx.send(HotPlugEvent::UsbAttached((id, descr.into()))).unwrap(),
            Err(e) => {
                error!(
                    "Error getting device descriptor for just arrived device {:?}: {}",
                    device, e
                );
            }
        };
    }

    fn device_left(&mut self, device: Device<T>) {
        let id = (&device).into();
        self.tx.send(HotPlugEvent::Detached(id)).unwrap();
    }
}

impl HotPlugHandler {
    pub fn try_setup(ctx: &rusb::Context, tx: channel::Sender<HotPlugEvent>) -> anyhow::Result<Self> {
        let handler = Self { tx };
        if rusb::has_hotplug() {
            ctx.register_callback(None, None, None, Box::new(handler.clone()))?;
            info!("Registered hotplug callback");
        } else {
            error!("Local libusb doesn't support hotplug. Going for manual detection ..");
            let handler_c = handler.clone();
            let ctx_c = ctx.clone();
            // TODO: shutdown thread, maybe ..
            std::thread::spawn(move || {
                let mut known_devices = Default::default();
                loop {
                    if let Err(e) = handler_c.poor_mans_hotplug_detection(&ctx_c, &mut known_devices) {
                        error!("Hotplug detection failed: {}. Retrying in 5 s ..", e);
                        // Conservatively mark all readers as detached
                        let _ = handler_c.notify_readers_detached(known_devices.iter().map(|x| x.0.clone()));
                        known_devices = Default::default();
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            });
        }
        let handler_c = handler.clone();
        // TODO: shutdown thread, maybe ..
        std::thread::spawn(move || {
            let mut known_readers = Default::default();
            loop {
                match pcsc::Context::establish(pcsc::Scope::User) {
                    Ok(pcsc_ctx) => {
                        if let Err(e) = handler_c.pcsc_detection_loop(&pcsc_ctx, &mut known_readers) {
                            error!("Hotplug pcsc detection failed: {}. Retrying in 1 s .. Note: This is normal on Windows on device ejection.", e);
                            // Conservatively mark all readers as detached
                            let _ = handler_c
                                .notify_readers_detached(known_readers.iter().map(|x| ReaderId::from_pcsc_name(&**x)));
                            known_readers = Default::default();
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    }
                    Err(e) => {
                        error!("Error creating pcsc context {}", e);
                        std::thread::sleep(Duration::from_secs(5));
                    }
                }
            }
        });
        Ok(handler)
    }
    /// Loop monitoring attached USB devices. Used in case `libusb` doesn't
    /// provide said capability (like on Windows).
    fn poor_mans_hotplug_detection(
        &self,
        ctx: &rusb::Context,
        known_devices: &mut BTreeMap<ReaderId, UsbDescriptor>,
    ) -> anyhow::Result<()> {
        // Monitor the list of attached usb devices
        loop {
            let mut new_devices: BTreeMap<ReaderId, UsbDescriptor> = Default::default();
            for device in ctx.devices()?.iter() {
                let id = (&device).try_into()?;
                let descriptor = device.device_descriptor()?;
                new_devices.insert(id, descriptor.into());
            }

            let old_keys: BTreeSet<_> = known_devices.keys().collect();
            let new_keys: BTreeSet<_> = new_devices.keys().collect();
            let added = new_keys.difference(&old_keys);
            let removed = old_keys.difference(&new_keys);
            added
                .map(|id| {
                    HotPlugEvent::UsbAttached((
                        (*id).clone(),
                        new_devices.get(*id).expect("Just read from it. qed.").clone(),
                    ))
                })
                .chain(removed.map(|id| HotPlugEvent::Detached((*id).clone())))
                .try_for_each(|x| self.tx.send(x))?;

            let _ = std::mem::replace(known_devices, new_devices);
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    /// Loop monitoring connected PCSC devices
    fn pcsc_detection_loop(&self, ctx: &pcsc::Context, known_readers: &mut BTreeSet<CString>) -> anyhow::Result<()> {
        let mut readers_buf = [0; 2048];
        loop {
            let new_readers = ctx
                .list_readers(&mut readers_buf)?
                .into_iter()
                // .map(|x| x.to_bytes().to_vec())
                .collect::<BTreeSet<_>>();

            let known = known_readers.iter().map(|x| &**x).collect();
            let added = new_readers.difference(&known);
            let removed = known.difference(&new_readers);

            added
                .map(|s| ReaderId::from_pcsc_name(*s))
                .map(|id| HotPlugEvent::PcscAttached((id, ctx.clone())))
                .chain(
                    removed
                        .map(|s| ReaderId::from_pcsc_name(*s))
                        .map(HotPlugEvent::Detached),
                )
                .try_for_each(|x| self.tx.send(x))?;

            let _ = std::mem::replace(known_readers, new_readers.iter().map(|x| (*x).to_owned()).collect());
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    fn notify_readers_detached<T>(&self, readers: T) -> anyhow::Result<()>
    where
        T: IntoIterator,
        T::Item: Into<ReaderId>,
    {
        for r in readers {
            self.tx.send(HotPlugEvent::Detached(r.into()))?;
        }
        Ok(())
    }
}
