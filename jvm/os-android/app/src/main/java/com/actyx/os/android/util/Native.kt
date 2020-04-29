package com.actyx.os.android.util

import com.sun.jna.Library
import com.sun.jna.Native as JnaNative

/**
 * Some syntactic sugar + logging for loading native libraries via JNA
 */
object Native {
  val log = Logger()

  inline fun <reified T : Library> loadLibrary(name: String): T {
    try {
      log.info("loading native {} library", name)
      val clazz = T::class.java
      return JnaNative.load<T>(name, clazz)
    } catch (t: Throwable) {
      log.error("error loading $name library: ${t.message}", t)
      throw t
    }
  }
}
