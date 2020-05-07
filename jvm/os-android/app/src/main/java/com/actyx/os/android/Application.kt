package com.actyx.os.android

import android.app.Application
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.service.IBackgroundServices
import com.actyx.os.android.util.ActivityLifecycleLogging
import com.actyx.os.android.util.Logger
import com.actyx.os.android.util.Rx
import org.slf4j.Logger
import java.io.File
import kotlin.system.exitProcess

class Application : Application() {

  lateinit var log: Logger

  private lateinit var activityLifecycleLogging: ActivityLifecycleLogging

  var backgroundServices: IBackgroundServices? = null
  private val backgroundServicesConnection = object : ServiceConnection {
    override fun onServiceConnected(name: ComponentName, service: IBinder) {
      backgroundServices = IBackgroundServices.Stub.asInterface(service)
    }

    override fun onServiceDisconnected(name: ComponentName?) {
      backgroundServices = null
    }
  }

  override fun onTerminate() {
    super.onTerminate()
    log.debug("applicationLifecycle:onTerminate")
    unregisterActivityLifecycleCallbacks(activityLifecycleLogging)
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

  override fun onCreate() {
    super.onCreate()

    // calling getExternalFilesDir also creates it making sure logback can write there
    // TODO: escalate when this returns null
    val extFilesDir = baseContext.getExternalFilesDir(null)
    File(extFilesDir, "logs").mkdir()
    log = Logger()

    log.debug("applicationLifecycle:onCreate")
    activityLifecycleLogging = ActivityLifecycleLogging()
    registerActivityLifecycleCallbacks(activityLifecycleLogging)

    Thread.setDefaultUncaughtExceptionHandler { thread, throwable ->
      log.error(
        "Unhandled exception thrown from thread $thread",
        throwable
      )
      exitProcess(2)
    }
    Rx.setErrorHandler()

    bindService(
      Intent(this, BackgroundServices::class.java),
      backgroundServicesConnection,
      Context.BIND_AUTO_CREATE
    )
  }
}
