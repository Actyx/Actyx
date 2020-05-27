package com.actyx.os.android.legacy.usb

import android.app.Activity
import android.content.Intent
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import arrow.core.Option.Companion.fromNullable
import arrow.core.filterOrOther
import arrow.core.flatMap
import com.actyx.os.android.util.Logger

private sealed class Message {
  data class Info(val message: String) : Message()
  data class Warn(val message: String) : Message()
}

/**
 * Activity for receiving implicit `ACTION_USB_DEVICE_ATTACHED` intents which takes care
 * that permissions are granted and (if applicable) persisted.
 */
class BaltechEventReceiverActivity : Activity() {
  val log = Logger()

  override fun onResume() {
    super.onResume()

    fromNullable(intent)
      .toEither { Message.Info("Activity resumed without intent") }
      .filterOrOther(
        { it.action == UsbManager.ACTION_USB_DEVICE_ATTACHED },
        { Message.Warn("Not a usb attached action. Received $it.action") })
      .flatMap {
        log.info("USB attached intent received")
        fromNullable(it.getParcelableExtra<UsbDevice?>(UsbManager.EXTRA_DEVICE))
          .toEither { Message.Warn("Intent doesn't have a device") }
      }
      .fold({
        when (it) {
          is Message.Info -> log.info(it.message)
          is Message.Warn -> log.warn(it.message)
        }
      }, { device ->
        sendBroadcast(
          Intent(ACTION_READER_ATTACHED).apply { putExtra(UsbManager.EXTRA_DEVICE, device) }
        )
        log.info("Sent {} intent for {}", ACTION_READER_ATTACHED, device.deviceName)
      })

    finish()
  }

  companion object {
    const val ACTION_READER_ATTACHED = "com.actyx.os.android.hardware.usb.action.READER_ATTACHED"
  }
}
