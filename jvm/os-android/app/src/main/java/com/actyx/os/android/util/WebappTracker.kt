package com.actyx.os.android.util

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.IBinder
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleObserver
import androidx.lifecycle.OnLifecycleEvent
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.service.IBackgroundServices
import java.io.Closeable

class WebappTracker(
  private val ctx: Context,
  private val lifecycle: Lifecycle,
  private val appId: String
) : LifecycleObserver, ServiceConnection, Closeable {

  private var backgroundServices: IBackgroundServices? = null

  init {
    ctx.bindService(
      Intent(ctx, BackgroundServices::class.java),
      this,
      Context.BIND_AUTO_CREATE
    )
    lifecycle.addObserver(this)
  }

  override fun onServiceConnected(name: ComponentName, service: IBinder) {
    val backgroundServices = IBackgroundServices.Stub.asInterface(service)
    if (lifecycle.currentState.isAtLeast(Lifecycle.State.CREATED)) {
      // Lifecycle.Event.ON_CREATE was triggered before the services were available
      backgroundServices.onAppEnabled(appId)
    }
    this.backgroundServices = backgroundServices
  }

  override fun onServiceDisconnected(name: ComponentName?) {
    backgroundServices = null
  }

  @OnLifecycleEvent(Lifecycle.Event.ON_CREATE)
  fun onStarted() {
    // when backgroundServices == null this will be done in onServiceConnected
    backgroundServices?.onAppEnabled(appId)
  }

  @OnLifecycleEvent(Lifecycle.Event.ON_DESTROY)
  fun onStopped() {
    backgroundServices?.onAppDisabled(appId)
    backgroundServices?.onAppStopped(appId)
  }

  override fun close() {
    lifecycle.removeObserver(this)
    ctx.unbindService(this)
  }
}
