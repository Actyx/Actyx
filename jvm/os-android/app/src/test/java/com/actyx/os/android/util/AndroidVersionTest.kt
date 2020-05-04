package com.actyx.os.android.util

import org.junit.Assert.*
import org.junit.Test

class AndroidVersionTest {

  @Test
  fun versionName() {
    assertEquals("Android 6.0", AndroidVersion.versionName(23))
  }
}
