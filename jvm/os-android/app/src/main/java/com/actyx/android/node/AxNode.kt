package com.actyx.android.node

import android.content.Context
import com.sun.jna.Callback
import com.sun.jna.CallbackThreadInitializer
import com.sun.jna.Native
import com.sun.jna.Pointer

typealias AxNodeMessageHandler = (code: Int, msg: String) -> Unit

class AxNode(ctx: Context, handler: AxNodeMessageHandler) {
  private val path: String = ctx.getExternalFilesDir(null)!!.path
  private val lib = com.actyx.android.util.Native.loadLibrary<AxNodeFFI>("axosnodeffi")
  private val callback = object : Callback {
    @Suppress("unused") // must contain a single method, name doesn't matter
    fun invoke(code: Int, msgPointer: Pointer) {
      try {
        val msg = msgPointer.getString(0, "utf8")
        handler(code, msg)
      } finally {
        lib.axnode_destroy_string(msgPointer)
      }
    }
  }
  private val initializer =
    CallbackThreadInitializer(true, false, "axnode_callback")

  init {
    Native.setCallbackThreadInitializer(callback, initializer)
    val err = RustError.ByReference()
    lib.axnode_init(path, callback, err)
    if (err.isSet()) {
      handler(err.code, err.consumeErrorMessage(lib))
    }
  }

  fun shutdownByUser() = lib.axnode_shutdown(0)
  fun shutdownBySystem() = lib.axnode_shutdown(1)
}
