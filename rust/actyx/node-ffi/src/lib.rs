use crossbeam::channel::bounded;
use ffi_support::{ErrorCode, ExternError, FfiStr};
use lazy_static::lazy_static;
use node::{spawn_with_name, ApplicationState, BindTo, NodeError, Runtime, ShutdownReason};
use parking_lot::Mutex;
use std::{convert::TryFrom, os::raw::c_char, sync::Arc};
use tracing::*;

lazy_static! {
    static ref STATE: Mutex<Option<ApplicationState>> = Mutex::new(None);
}
ffi_support::define_string_destructor!(axnode_destroy_string);
/// Registered callback for conveying messages. The receiving side needs to make
/// sure to eventually free the memory again. This lib provides a destructor for
/// the string: `axnode_destroy_string`.
/// The message codes are defined in `node/src/components/android.rs`.
type Callback = unsafe extern "C" fn(i32, *mut c_char) -> ();
#[no_mangle]
/// Main entry point for initialization of this library. All files (even temporary ones) will only
/// be created under `working_dir`.
/// A callback must be installed, with which messages are conveyed across the FFI
/// boundary.
pub extern "C" fn axnode_init(working_dir: FfiStr, callback: Callback, error: &mut ExternError) {
    ffi_support::call_with_result(error, || {
        callback_holder::set_callback(callback);
        let (ffi_sink, rx) = bounded(32);
        let mut state = STATE.lock();
        if state.is_none() {
            match ApplicationState::spawn(
                working_dir.as_str().into(),
                Runtime::Android { ffi_sink },
                BindTo::default(),
            ) {
                Ok(handle) => {
                    *state = Some(handle);
                    spawn_with_name("ffi_sink", move || loop {
                        if let Ok(msg) = rx.recv() {
                            trace!("Sending over ffi: {:?}", msg);
                            let (code, c_str) = msg.into();
                            callback_holder::send(code, c_str);
                        }
                    });
                    Ok(())
                }
                Err(e) => Err(ExternError::new_error(ErrorCode::new(42), format!("{:?}", e))),
            }
        } else {
            Err(ExternError::new_error(ErrorCode::new(42), "Thou shalt not init twice"))
        }
    })
}

#[no_mangle]
/// Integer indicates whether the system or the user triggered the shutdown.
pub extern "C" fn axnode_shutdown(shutdown_reason: i32) {
    if let Some(mut handle) = STATE.lock().take() {
        // `ApplicationState::Drop` will do the right thing
        handle.shutdown(
            ShutdownReason::try_from(shutdown_reason)
                .unwrap_or_else(|_| ShutdownReason::Internal(NodeError::InternalError(Arc::new(anyhow::anyhow!(""))))),
        )
    }
}

// https://github.com/mozilla/application-services/blob/db28a39663bf117392c6aafb9e43e4411f395535/components/viaduct/src/backend/ffi.rs
/// Module that manages get/set of the global Android callback pointer.
pub(crate) mod callback_holder {
    use super::Callback;
    use ffi_support::destroy_c_string;
    use std::{
        os::raw::c_char,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use tracing::{error, warn};

    /// Note: We only assign to this once.
    static CALLBACK_PTR: AtomicUsize = AtomicUsize::new(0);

    /// Get the function pointer to the Callback, if initialized.
    pub(super) fn get_callback() -> Option<Callback> {
        let ptr_value = CALLBACK_PTR.load(Ordering::SeqCst);
        unsafe { std::mem::transmute::<usize, Option<Callback>>(ptr_value) }
    }

    /// Sends ByteBuffer over the FFI barrier, using a callback, if registered.
    /// Will drop the buffer, if no callback is registered.
    pub(super) fn send(code: i32, message: *mut c_char) {
        if let Some(cb) = get_callback() {
            unsafe { cb(code, message) };
        } else {
            warn!("Dropping message, as no callback is registered");
            unsafe { destroy_c_string(message) };
        }
    }

    /// Set the function pointer to the Callback. Returns false if we did nothing because the callback had already been initialized
    pub(super) fn set_callback(h: Callback) -> bool {
        let as_usize = h as usize;
        #[allow(deprecated)]
        let old_ptr = CALLBACK_PTR.compare_and_swap(0, as_usize, Ordering::SeqCst);
        if old_ptr != 0 {
            // This is an internal bug, the other side of the FFI should ensure
            // it sets this only once.
            error!("Bug: Initialized CALLBACK_PTR multiple times");
        }
        old_ptr == 0
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ffi_support::rust_string_to_c;
        use lazy_static::lazy_static;

        #[test]
        fn should_not_panic() {
            get_callback();
        }
        lazy_static! {
            static ref CALLBACK_CALLED: AtomicUsize = AtomicUsize::default();
        }

        extern "C" fn callback(_: i32, _: *mut c_char) {
            CALLBACK_CALLED.fetch_add(1, Ordering::SeqCst);
        }

        #[test]
        fn should_work() {
            assert_eq!(CALLBACK_CALLED.fetch_add(0, Ordering::SeqCst), 0);
            assert_eq!(CALLBACK_CALLED.fetch_add(0, Ordering::SeqCst), 0);
            assert!(set_callback(callback));
            assert_eq!(CALLBACK_CALLED.fetch_add(0, Ordering::SeqCst), 0);
            if get_callback().is_none() {
                panic!()
            }
            send(0, rust_string_to_c("rust_string".to_string()));
            assert_eq!(CALLBACK_CALLED.fetch_add(0, Ordering::SeqCst), 1);
        }
    }
}
