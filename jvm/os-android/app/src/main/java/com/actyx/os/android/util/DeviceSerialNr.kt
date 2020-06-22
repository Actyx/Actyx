package com.actyx.os.android.util

import android.annotation.SuppressLint
import android.content.Context
import android.os.Build
import android.provider.Settings

object DeviceSerialNr {

  @SuppressLint("HardwareIds")
  fun get(context: Context?): String =
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      // needs READ_PRIVILEGED_PHONE_STATE permission which is only available to system apps see
      // https://developer.android.com/about/versions/10/privacy/changes#non-resettable-device-ids
      context?.let {
        Settings.Secure.getString(it.contentResolver, Settings.Secure.ANDROID_ID)
      } ?: Build.UNKNOWN
    } else {
      Build.SERIAL
    }
}
