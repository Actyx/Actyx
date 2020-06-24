package com.actyx.os.android.activity

import android.app.ActivityManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Bundle
import androidx.activity.viewModels
import androidx.core.content.ContextCompat

import com.actyx.os.android.R
import com.actyx.os.android.service.BackgroundServices.Companion.ACTION_APP_LIST_UPDATED
import com.actyx.os.android.service.IBackgroundServices
import com.actyx.os.android.util.toBitmap
import com.actyx.os.android.viewmodel.AppInfosViewModel

class MainActivity : BaseActivity() {

  private var backgroundServices: IBackgroundServices? = null
  private val appInfos: AppInfosViewModel by viewModels()

  private val appsUpdatedReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context?, intent: Intent?) {
      updateAppList()
    }
  }

  override fun onStart() {
    super.onStart()
    registerReceiver(appsUpdatedReceiver, IntentFilter(ACTION_APP_LIST_UPDATED))
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContentView(R.layout.activity_main)
    setupNavigation()
    setTaskDescription()
  }

  fun updateAppList() {
    backgroundServices?.let {
      appInfos.setAppInfos(it.apps)
    }
  }

  override fun onBackgroundServicesConnected(backgroundServices: IBackgroundServices) {
    log.debug("onBackgroundServicesConnected")
    this.backgroundServices = backgroundServices
    updateAppList()
  }

  override fun onBackgroundServicesDisconnected() {
    log.debug("onBackgroundServicesDisonnected")
    this.backgroundServices = null
  }

  override fun onDestroy() {
    super.onDestroy()
    unregisterReceiver(appsUpdatedReceiver)
  }

  @Suppress("DEPRECATION")
  private fun setTaskDescription() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
      val name = resources.getString(R.string.app_name)
      val icon = ContextCompat.getDrawable(this, R.drawable.ic_actyxos_circle)?.toBitmap()
      val color = ContextCompat.getColor(this, R.color.colorPrimary)
      //noinspection deprecation
      setTaskDescription(ActivityManager.TaskDescription(name, icon, color))
    }
  }

  companion object {
    const val ACTYXOS_APP_MIME_TYPE = "actyxos/app"
    const val ACTION_SHORTCUT_ADDED_CALLBACK = "actyxos.intent.action.SHORTCUT_ADDED_CALLBACK"
    const val REQ_CODE_SHORTCUT_ADDED_CALLBACK = 0
  }
}
