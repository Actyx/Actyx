use crate::ReaderId;
use crossbeam::channel;
use rusb::{Device, DeviceDescriptor, UsbContext};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryInto,
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
            std::thread::spawn(move || loop {
                if let Err(e) = handler_c.poor_mans_hotplug_detection(&ctx_c) {
                    error!("Hotplug detection failed: {}. Retrying in 5 s ..", e);
                    std::thread::sleep(Duration::from_secs(5));
                }
            });
        }
        let handler_c = handler.clone();
        // TODO: shutdown thread, maybe ..
        std::thread::spawn(move || loop {
            match pcsc::Context::establish(pcsc::Scope::User) {
                Ok(pcsc_ctx) => {
                    if let Err(e) = handler_c.pcsc_detection_loop(&pcsc_ctx) {
                        error!("Hotplug pcsc detection failed: {}. Retrying in 1 s .. Note: This is normal on Windows on device ejection.", e);
                        std::thread::sleep(Duration::from_secs(1));
                    }
                }
                Err(e) => {
                    error!("Error creating pcsc context {}", e);
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        });
        Ok(handler)
    }
    /// Loop monitoring attached USB devices. Used in case `libusb` doesn't
    /// provide said capability (like on Windows).
    fn poor_mans_hotplug_detection(&self, ctx: &rusb::Context) -> anyhow::Result<()> {
        let mut known_devices: BTreeMap<ReaderId, UsbDescriptor> = Default::default();
        // Get status quo
        for device in ctx.devices()?.iter() {
            let id = (&device).try_into()?;
            let descriptor = device.device_descriptor()?;
            known_devices.insert(id, descriptor.into());
        }
        // Now monitor the list of attached usb devices
        loop {
            std::thread::sleep(Duration::from_millis(50));
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

            known_devices = new_devices;
        }
    }
    /// Loop monitoring connected PCSC devices
    fn pcsc_detection_loop(&self, ctx: &pcsc::Context) -> anyhow::Result<()> {
        use pcsc::*;
        let mut readers_buf = [0; 2048];
        let mut reader_states = vec![
            // Listen for reader insertions/removals, if supported.
            ReaderState::new(PNP_NOTIFICATION(), State::UNAWARE),
        ];
        loop {
            // Remove dead readers.
            fn is_dead(rs: &ReaderState) -> bool {
                rs.event_state().intersects(State::UNKNOWN | State::IGNORE)
            }
            for rs in &reader_states {
                if is_dead(rs) {
                    info!("PCSC: Removing {:?}", rs.name());
                    let id = rs.into();
                    self.tx.send(HotPlugEvent::Detached(id)).unwrap();
                }
            }
            reader_states.retain(|rs| !is_dead(rs));

            // Add new readers.
            let names = ctx.list_readers(&mut readers_buf)?;
            for name in names {
                if !reader_states.iter().any(|rs| rs.name() == name) {
                    info!("Adding {:?}", name);
                    let reader_state = ReaderState::new(name, State::UNAWARE);
                    let id = (&reader_state).into();
                    reader_states.push(reader_state);
                    self.tx.send(HotPlugEvent::PcscAttached((id, ctx.clone()))).unwrap();
                }
            }

            // Update the view of the state to wait on.
            for rs in &mut reader_states {
                rs.sync_current_state();
            }

            // Wait until the state changes.
            // This blocks the complete lib, so we need to yield from time to time (timeout)
            if let Err(e) = ctx.get_status_change(Duration::from_millis(50), &mut reader_states[..]) {
                if e != pcsc::Error::Timeout {
                    error!("Error getting status change: {}", e);
                }
            }
        }
    }
}
