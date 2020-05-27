package com.actyx.os.android.osnode

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Pointer
import mozilla.appservices.support.RustBuffer

@Suppress("FunctionNaming")
interface AxNodeFFI : Library {
  /**
   * Expects a pointer to an instance of ToNative which will become invalid once this function
   * returns.
   */
  fun axosnode_send(data: Pointer, len: Int, out_err: RustError.ByReference)

  /**
   * Desctructs the given buffer.
   *
   * see https://docs.rs/ffi-support/0.4.0/ffi_support/macro.define_bytebuffer_destructor.html
   */
  fun axosnode_destroy_bytebuffer(bb: RustBuffer.ByValue)

  /**
   * Desctructs the given string.
   *
   * see https://docs.rs/ffi-support/0.4.0/ffi_support/macro.define_string_destructor.html
   */
  fun axosnode_destroy_string(str: Pointer)

  /**
   * Initializes the native node backend.
   */
  fun axosnode_init(working_dir: String, cb: Callback, out_err: RustError.ByReference)

  /**
   * Retrieves the currently set settings.
   */
  fun axosnode_get_settings(
    scope: String,
    no_defaults: Boolean,
    err: RustError.ByReference
  ): String
}
