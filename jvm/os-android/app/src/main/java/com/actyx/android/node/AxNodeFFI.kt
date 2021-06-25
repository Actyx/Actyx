package com.actyx.android.node

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Pointer

@Suppress("FunctionName")
interface AxNodeFFI : Library {

  /**
   * Desctructs the given string.
   *
   * see https://docs.rs/ffi-support/0.4.0/ffi_support/macro.define_string_destructor.html
   */
  fun axnode_destroy_string(str: Pointer)

  /**
   * Initializes the native node backend.
   */
  fun axnode_init(working_dir: String, cb: Callback, out_err: RustError.ByReference)

  /**
   * Shutdown node
   */
  fun axnode_shutdown(exit_code: Int)
}
