package com.actyx.android

import android.app.Application
import android.content.Intent
import android.os.Build
import android.os.Environment
import com.actyx.android.util.Logger
import org.slf4j.Logger
import java.io.File
import kotlin.system.exitProcess

class ActyxApplication : Application() {
  private lateinit var log: Logger

  fun migrateFromV1(): Boolean {
    // Asserting that a file exists works even without `android.permission.READ_EXTERNAL_STORAGE`
    val v1bundle =
      File(Environment.getExternalStorageDirectory(), V1MigrationActivity.V1BundleFileName).exists()
    val dataDirPopulated = File(baseContext.getExternalFilesDir(null), "node.sqlite").exists()

    if (!dataDirPopulated && v1bundle) {
      log.info("Found ActyxOS v1")
      val intent = Intent(this, V1MigrationActivity::class.java)
      intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      startActivity(intent)
      return true
    } else {
      return false
    }
  }

  override fun onCreate() {
    super.onCreate()
    // calling getExternalFilesDir also creates it making sure logback can write there
    // TODO: refactor when targeting Android 11
    val extFilesDir = baseContext.getExternalFilesDir(null)
    File(extFilesDir, "logs").mkdir()
    log = Logger()
    log.info("applicationLifecycle:onCreate")
    Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
      log.error("Unhandled exception thrown from thread $thread", throwable)
      exitProcess(2)
    }

    if (!migrateFromV1()) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
          startForegroundService(Intent(this, AxNodeService::class.java))
        } else {
          startService(Intent(this, AxNodeService::class.java))
        }
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
