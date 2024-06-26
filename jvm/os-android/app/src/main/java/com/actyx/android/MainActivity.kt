package com.actyx.android

import android.annotation.SuppressLint
import android.app.ActivityManager
import android.content.Intent
import android.content.pm.ActivityInfo
import android.net.ConnectivityManager
import android.net.LinkProperties
import android.os.Build
import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.actyx.android.util.Logger
import com.actyx.android.util.toBitmap
import java.net.Inet4Address
import kotlin.system.exitProcess

class MainActivity : AppCompatActivity() {
  private val log = Logger()
  private var shutdownPending = false

  @SuppressLint("SourceLockedOrientationActivity")
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    log.info("onCreate: $intent")
    setContentView(R.layout.activity_main)
    if (!resources.getBoolean(R.bool.isTablet)) {
      this.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
    }
    setTaskDescription()

    findViewById<Button>(R.id.quit_actyx)?.setOnClickListener {
      AlertDialog.Builder(this)
        .setMessage(getString(R.string.quit_ax_confirm_message))
        .setPositiveButton(getString(R.string.quit)) { _, _ -> signalOrForceShutdown() }
        .setNegativeButton(getString(R.string.cancel)) { dialog, _ -> dialog.cancel() }
        .create().apply {
          setTitle(getString(R.string.quit_ax_confirm_title))
          show()
        }
    }
  }

  override fun onNewIntent(intent: Intent?) {
    super.onNewIntent(intent)
    if (intent != null && intent.hasExtra(AxNodeService.AXNODE_MESSAGE)) {
      val message = intent.getStringExtra(AxNodeService.AXNODE_MESSAGE)
      log.info("Actyx is stopped. $message")
      // User initiated shutdown, so skip alert dialog
      if (shutdownPending) {
        finish()
        exitProcess(0)
      } else {
        AlertDialog.Builder(this)
          .setCancelable(false)
          .setMessage(message)
          .setPositiveButton(getString(R.string.ok)) { _, _ ->
            finish()
            exitProcess(0)
          }
          .create().apply {
            setTitle(getString(R.string.actyx_is_stopped))
            show()
          }
      }
    } else if (intent != null && intent.action == AxNodeService.REQUEST_QUIT) {
      findViewById<Button>(R.id.quit_actyx)?.callOnClick()
    }
  }

  override fun onResume() {
    super.onResume()
    val ipTextView: TextView = findViewById(R.id.ip_address_value)
    ipTextView.text = getIPAddress()
  }

  private fun signalOrForceShutdown() {
    if (shutdownPending) {
      // Shutdown is already pending, maybe the background service is not responsive.
      Toast.makeText(this, getString(R.string.actyx_is_force_stopping), Toast.LENGTH_SHORT).show()
      Intent(this@MainActivity, AxNodeService::class.java).also {
        stopService(it)
      }
      finish()
      exitProcess(1)
    }
    shutdownPending = true
    Toast.makeText(this, getString(R.string.actyx_is_stopping), Toast.LENGTH_SHORT).show()
    Intent(this@MainActivity, AxNodeService::class.java).also {
      it.action = AxNodeService.STOPFOREGROUND_ACTION
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        startForegroundService(it)
      } else {
        startService(it)
      }
    }
  }

  @Suppress("DEPRECATION")
  private fun setTaskDescription() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
      val name = resources.getString(R.string.app_name)
      val icon = ContextCompat.getDrawable(this, R.drawable.actyx_icon)?.toBitmap()
      val color = ContextCompat.getColor(this, R.color.gray_dark)
      //noinspection deprecation
      setTaskDescription(ActivityManager.TaskDescription(name, icon, color))
    }
  }

  private fun isConnected(cm: ConnectivityManager): Boolean =
    cm.activeNetworkInfo?.isConnectedOrConnecting == true

  private fun getLinkProps(cm: ConnectivityManager): LinkProperties? =
    cm.getLinkProperties(cm.activeNetwork)

  private fun getIPAddress(): String =
    getSystemService(ConnectivityManager::class.java)?.let { cm ->
      if (isConnected(cm)) {
        getLinkProps(cm)?.linkAddresses
          ?.map { it.address }
          ?.filterIsInstance<Inet4Address>()
          ?.first()
          ?.hostAddress
      } else null
    } ?: getString(R.string.ip_not_available)
}
