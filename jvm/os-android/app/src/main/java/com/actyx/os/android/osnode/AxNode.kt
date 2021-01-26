package com.actyx.os.android.osnode

import android.content.Context
import arrow.core.Either
import arrow.core.left
import arrow.core.right
import com.actyx.os.android.osnode.model.NodeFfi.ToAndroid
import com.actyx.os.android.osnode.model.NodeFfi.ToNative
import com.actyx.os.android.util.Logger
import com.sun.jna.Callback
import com.sun.jna.CallbackThreadInitializer
import com.sun.jna.Native
import mozilla.appservices.support.RustBuffer
import mozilla.appservices.support.toNioDirectBuffer

typealias Scope = String
typealias MessageHandler = (ToAndroid) -> Unit

class AxNode(ctx: Context, handler: MessageHandler) {
  val log = Logger()
  var lastNodeState = ToAndroid.NodeStateChanged.State.MISCONFIGURED

  val path: String = ctx.getExternalFilesDir(null)!!.path

  private val lib = com.actyx.os.android.util.Native.loadLibrary<AxNodeFFI>("axosnodeffi")

  private val callback = object : Callback {
    @Suppress("unused") // must contain a single method, name doesn't matter
    fun invoke(rustBuffer: RustBuffer.ByValue) {
      try {
        val msg = ToAndroid.parseFrom(rustBuffer.asCodedInputStream()!!)
        if (msg.payloadCase == ToAndroid.PayloadCase.NODESTATECHANGED) {
          lastNodeState = msg.nodeStateChanged.state
        }
        handler(msg)
        log.debug("From native {} {}", msg.payloadCase, msg)
      } finally {
        lib.axosnode_destroy_bytebuffer(rustBuffer)
      }
    }
  }
  private val initializer =
    CallbackThreadInitializer(true, false, "axosnode_callback")

  init {
    Native.setCallbackThreadInitializer(callback, initializer)
    log.info("Initializing native platform lib. path={}", path)
    call { err -> lib.axosnode_init(path, callback, err) }
    log.info("Initialization of native platform lib done.")
  }

  fun getSettings(scope: Scope, includeDefaults: Boolean = true): Either<String, String> {
    if (lastNodeState == ToAndroid.NodeStateChanged.State.MISCONFIGURED) {
      return "Node is misconfigured".left()
    }
    return call { err -> lib.axosnode_get_settings(scope, !includeDefaults, err) }
  }

  fun notifyAppStarted(appId: String) {
  }

  fun notifyAppStopped(appId: String) {
  }

  fun notifyAppEnabled(appId: String) {
  }

  fun notifyAppDisabled(appId: String) {
  }

  fun sendMessage(msg: ToNative): Either<String, Unit> {
    log.debug("Sending to native lib: {}", msg)
    val (nioBuf, len) = msg.toNioDirectBuffer()
    return call { err ->
      val ptr = Native.getDirectBufferPointer(nioBuf)
      lib.axosnode_send(ptr, len, err)
    }
  }

  private inline fun <T> call(
    cb: (RustError.ByReference) -> T
  ): Either<String, T> {
    val err = RustError.ByReference()
    val result = cb(err)
    return if (err.isFailure()) {
      err.consumeErrorMessage(lib).left()
    } else {
      result.right()
    }
  }
}
