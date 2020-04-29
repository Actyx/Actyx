package com.actyx.os.android.activity

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Bundle
import android.os.IBinder
import androidx.appcompat.app.AppCompatActivity
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.service.IBackgroundServices
import com.actyx.os.android.util.Logger

abstract class BaseActivity : AppCompatActivity() {
  val log = Logger()

  private val backgroundServicesConnection = object : ServiceConnection {
    override fun onServiceConnected(name: ComponentName, service: IBinder) {
      val backgroundServices = IBackgroundServices.Stub.asInterface(service)
      onBackgroundServicesConnected(backgroundServices)
    }

    override fun onServiceDisconnected(name: ComponentName?) {
      onBackgroundServicesDisconnected()
    }
  }

  protected abstract fun onBackgroundServicesConnected(backgroundServices: IBackgroundServices)

  protected abstract fun onBackgroundServicesDisconnected()

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    log.info("binding background services")
    bindService(
      Intent(this, BackgroundServices::class.java),
      backgroundServicesConnection,
      Context.BIND_AUTO_CREATE
    )
  }

  override fun onDestroy() {
    super.onDestroy()
    log.info("unbinding background services")
    unbindService(backgroundServicesConnection)
  }
}
