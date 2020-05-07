package com.actyx.os.android.util

import java.io.FilterInputStream
import java.io.InputStream
import java.io.OutputStream
import java.io.PipedInputStream
import java.io.PipedOutputStream

/**
 * returns an InputStream that mirrors what got written to the OutputStream provided to the callback
 */
fun toInputStream(write: (OutputStream) -> Unit): InputStream {
  val sink = PipedInputStream()
  val src = PipedOutputStream(sink)
  val lock = Object()
  // From PipedOutputStream:
  // "Attempting to use both objects from a single thread is not recommended as it may deadlock the
  // thread."
  Thread(Runnable {
    write(src)
    synchronized(lock) {
      try {
        // From PipedOutputStream:
        // "A pipe is said to be broken if a thread that was providing data bytes to the connected
        // piped output stream is no longer alive."
        lock.wait()
      } catch (e: InterruptedException) {
        e.printStackTrace()
      }
    }
  }).start()
  return object : FilterInputStream(sink) {
    override fun close() {
      super.close()
      synchronized(lock) {
        lock.notifyAll()
      }
    }
  }
}
