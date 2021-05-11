package com.actyx.android

import android.app.Application
import android.content.Intent
import android.os.Build
import com.actyx.android.util.Logger
import org.slf4j.Logger
import java.io.File
import kotlin.system.exitProcess

class ActyxApplication : Application() {
  lateinit var log: Logger

  override fun onCreate() {
    super.onCreate()
    // calling getExternalFilesDir also creates it making sure logback can write there
    // TODO: escalate when this returns null
    val extFilesDir = baseContext.getExternalFilesDir(null)!!
    File(extFilesDir, "logs").mkdir()
    log = Logger()
    log.info("applicationLifecycle:onCreate")
    Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
      log.error("Unhandled exception thrown from thread $thread", throwable)
      exitProcess(2)
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      startForegroundService(Intent(this, AxNodeService::class.java))
    } else {
      startService(Intent(this, AxNodeService::class.java))
    }
  }

  override fun onLowMemory() {
    super.onLowMemory()
    log.warn("applicationLifecycle:onLowMemory")
  }

  override fun onTrimMemory(level: Int) {
    super.onTrimMemory(level)
    val levelStr = when (level) {
      TRIM_MEMORY_COMPLETE -> "TRIM_MEMORY_COMPLETE"
      TRIM_MEMORY_MODERATE -> "TRIM_MEMORY_MODERATE"
      TRIM_MEMORY_BACKGROUND -> "TRIM_MEMORY_BACKGROUND"
      TRIM_MEMORY_UI_HIDDEN -> "TRIM_MEMORY_UI_HIDDEN"
      TRIM_MEMORY_RUNNING_CRITICAL -> "TRIM_MEMORY_RUNNING_CRITICAL"
      TRIM_MEMORY_RUNNING_LOW -> "TRIM_MEMORY_RUNNING_LOW"
      TRIM_MEMORY_RUNNING_MODERATE -> "TRIM_MEMORY_RUNNING_MODERATE"
      else -> "???"
    }
    log.warn("applicationLifecycle:onTrimMemory, level: $levelStr")
  }
}
