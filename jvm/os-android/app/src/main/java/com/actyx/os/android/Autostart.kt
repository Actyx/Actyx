package com.actyx.os.android

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

class Autostart : BroadcastReceiver() {
  override fun onReceive(ctx: Context, intent: Intent?) {
    // Just a dummy receiver. Application.onCreate takes care of starting the services.
  }
}
