use crate::{baltech, AccessoryError};
use rusb::DeviceDescriptor;
use serde::Serialize;
use std::{ffi::CString, fmt, time::Duration};
use tracing::*;

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderId {
    pub friendly_name: String,
    #[serde(skip_serializing)]
    protocol_identifier: Vec<u8>,
}
impl fmt::Display for ReaderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.friendly_name)
    }
}

const BALTECH_VENDOR_ID: u16 = 0x13ad;
pub enum CardReader {
    /// Custom Baltech card reader interface
    Baltech { id: ReaderId, context: baltech::Context },
    /// Generic PCSC provided card reader interface
    PCSC {
        id: ReaderId,
        context: pcsc::Context,
        reader_state: pcsc::ReaderState,
    },
}
impl CardReader {
    pub fn create_from_usb(id: ReaderId, usb_descriptor: &DeviceDescriptor) -> anyhow::Result<Self> {
        if usb_descriptor.vendor_id() == BALTECH_VENDOR_ID {
            let context = baltech::Context::open()?;
            Ok(CardReader::Baltech { id, context })
        } else {
            Err(AccessoryError::NotABaltechReader.into())
        }
    }
    pub fn create_from_pcsc(context: pcsc::Context, id: ReaderId) -> Self {
        let reader_state = (&id).into();
        Self::PCSC {
            context,
            id,
            reader_state,
        }
    }
    /// Tries to read the UID from a card placed onto the reader. If `reselect`
    /// is set to false, the uid from an already processed card is read again (in
    /// other words, the card has not been touched/moved/lifted since the last
    /// time we read from it).
    pub fn try_read_uid(&mut self, reselect: bool) -> anyhow::Result<Vec<u8>> {
        match self {
            CardReader::Baltech { context, .. } => context.try_read_uid(reselect),
            CardReader::PCSC {
                context, reader_state, ..
            } => {
                let last_event_count = reader_state.event_count();
                trace!(
                    "Reader \"{:?}\": event count pre: {}",
                    reader_state.name(),
                    last_event_count
                );
                {
                    let mut new_reader_state = vec![pcsc::ReaderState::new(reader_state.name(), pcsc::State::UNAWARE)];
                    // This should be instantaneous. There's something wrong if it takes longer.
                    context.get_status_change(Duration::from_millis(50), &mut new_reader_state)?;
                    std::mem::swap(reader_state, &mut new_reader_state[0]);
                    reader_state.sync_current_state();
                }
                let new_event_count = reader_state.event_count();
                trace!(
                    "Reader \"{:?}\" event count post: {}",
                    reader_state.name(),
                    new_event_count
                );

                if !reselect && (last_event_count == new_event_count) {
                    trace!("No or already scanned card still present, ignoring.");
                    return Err(AccessoryError::NoCardPresent.into());
                }

                let card = context
                    .connect(reader_state.name(), pcsc::ShareMode::Shared, pcsc::Protocols::ANY)
                    .map_err(|err| match err {
                        pcsc::Error::NoSmartcard | pcsc::Error::Cancelled => AccessoryError::NoCardPresent,
                        err => AccessoryError::PcscError(err),
                    })?;

                let apdu = vec![0xFF, 0xCA, 0x00, 0x00, 0x00];
                trace!("Sending APDU: {:?}", apdu);
                let mut rapdu_buf = [0; pcsc::MAX_BUFFER_SIZE];
                let mut resp = card
                    .transmit(apdu.as_slice(), &mut rapdu_buf)
                    .map_err(|err| {
                        error!("Failed to transmit APDU command to card: {}", err);
                        AccessoryError::PcscError(err)
                    })?
                    .to_vec();
                // TODO: maybe need to cut last two bytes? http://downloads.acs.com.hk/drivers/en/API-ACR122U-2.02.pdf
                // at least for ACR122uu last two bytes indicate success (0x90 0x00); need to check Zebra scanner
                if resp.len() < 4 {
                    error!("Error from running APDU command: {:X?}", resp);
                    Err(AccessoryError::PcscError(pcsc::Error::UnknownResMng).into())
                } else {
                    Ok(resp.drain(..4).collect())
                }
            }
        }
    }
}

fn get_friendly_name<T: rusb::UsbContext>(d: &rusb::Device<T>) -> anyhow::Result<String> {
    let descriptor = d.device_descriptor()?;
    let handle = d.open()?;
    let manufacturer = handle.read_manufacturer_string_ascii(&descriptor)?;
    let product = handle.read_product_string_ascii(&descriptor)?;
    let sn = handle.read_serial_number_string_ascii(&descriptor)?;
    Ok(format!("{}:{} ({})", manufacturer, product, sn))
}
impl<T: rusb::UsbContext> From<&rusb::Device<T>> for ReaderId {
    fn from(d: &rusb::Device<T>) -> Self {
        let protocol_identifier = vec![d.bus_number(), d.address(), d.port_number()];
        let friendly_name = get_friendly_name(d).unwrap_or_else(|_| "Unknown".to_string());
        Self {
            friendly_name,
            protocol_identifier,
        }
    }
}

impl From<&pcsc::ReaderState> for ReaderId {
    fn from(rs: &pcsc::ReaderState) -> Self {
        let protocol_identifier = rs.name().to_bytes().to_vec();
        let friendly_name = rs.name().to_string_lossy().to_string();
        Self {
            protocol_identifier,
            friendly_name,
        }
    }
}
impl Into<pcsc::ReaderState> for &ReaderId {
    fn into(self) -> pcsc::ReaderState {
        let name = CString::new(self.protocol_identifier.clone())
            .expect("Tried to create ReaderState from different protocol_identifier!");
        pcsc::ReaderState::new(&*name, pcsc::State::UNAWARE)
    }
}
