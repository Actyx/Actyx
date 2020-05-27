package com.actyx.os.android.util

object AndroidVersion {
  // see https://source.android.com/setup/start/build-numbers
  private val apiLevelsToVersionNames = mapOf(
    29 to "10",
    28 to "9",
    27 to "8.1",
    26 to "8.0",
    25 to "7.1",
    24 to "7.0",
    23 to "6.0",
    22 to "5.1",
    21 to "5.0",
    19 to "4.4",
    18 to "4.3",
    17 to "4.2",
    16 to "4.1"
  )

  fun versionName(apiLevel: Int) =
    apiLevelsToVersionNames[apiLevel]?.let { "Android $it" }
}
