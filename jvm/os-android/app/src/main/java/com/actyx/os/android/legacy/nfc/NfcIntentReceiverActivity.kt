package com.actyx.os.android.legacy.nfc

import android.app.Activity
import android.content.Intent
import android.nfc.NfcAdapter
import android.nfc.Tag
import arrow.core.Option.Companion.fromNullable
import com.actyx.os.android.legacy.usb.bytesToHex
import com.actyx.os.android.util.Logger

class NfcIntentReceiverActivity : Activity() {
  val log = Logger()

  override fun onResume() {
    super.onResume()
    if (intent.action == NfcAdapter.ACTION_TECH_DISCOVERED) {
      fromNullable(intent.getParcelableExtra<Tag?>(NfcAdapter.EXTRA_TAG))
        .map { bytesToHex(it.id) }
        .fold({
          log.warn("Couldn't get NFC tag")
        }, {
          log.info("NFC read: {}", it)
          val broadcastIntent = Intent(NFCA_TAG_ID_SCANNED).apply { putExtra(EXTRA_TAG_ID, it) }
          sendBroadcast(broadcastIntent)
        })
    }

    finish()
  }

  companion object {
    const val NFCA_TAG_ID_SCANNED = "com.actyx.os.android.hardware.nfc.action.NFCA_TAG_ID_SCANNED"
    const val EXTRA_TAG_ID = "id"
  }
}
