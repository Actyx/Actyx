package com.actyx.android

import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent

class Autostart : BroadcastReceiver() {
  @SuppressLint("UnsafeProtectedBroadcastReceiver")
  override fun onReceive(ctx: Context, intent: Intent?) {
    // Just a dummy receiver. Application.onCreate takes care of starting the ax node service.
  }
}
