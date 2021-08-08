package com.actyx.android.node

import com.sun.jna.Pointer
import com.sun.jna.Structure

@Structure.FieldOrder("code", "message")
open class RustError : Structure() {

  class ByReference : RustError(), Structure.ByReference

  @JvmField
  var code: Int = 0
  @JvmField
  var message: Pointer? = null

  fun isSet(): Boolean {
    return code != 0
  }

  /**
   * Get and consume the error message.
   */
  fun consumeErrorMessage(lib: AxNodeFFI): String =
    this.getMessage()?.let { result ->
      lib.axnode_destroy_string(this.message!!)
      this.message = null
      result
    } ?: throw NullPointerException("consumeErrorMessage called with null message!")

  /**
   * Get the error message or null if there is none.
   */
  private fun getMessage(): String? {
    return this.message?.getString(0, "utf8")
  }
}
