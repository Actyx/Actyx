package com.actyx.os.android

import android.net.Uri
import android.os.Parcelable
import kotlinx.android.parcel.Parcelize

@Parcelize
data class AppInfo(
  val id: String,
  val version: String,
  val name: String,
  val iconPath: String?,
  val uri: Uri,
  val settingsSchema: String
) : Parcelable
