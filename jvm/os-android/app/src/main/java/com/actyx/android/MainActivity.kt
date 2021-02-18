package com.actyx.android

import android.annotation.SuppressLint
import android.app.ActivityManager
import android.content.Intent
import android.content.pm.ActivityInfo
import android.os.Build
import android.os.Bundle
import android.widget.Button
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.actyx.android.util.Logger

import com.actyx.android.util.toBitmap

class MainActivity : AppCompatActivity() {
  private val log = Logger()

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
        .setPositiveButton(getString(R.string.quit)) { _, _ -> signalShutdown() }
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
      AlertDialog.Builder(this)
        .setCancelable(false)
        .setMessage(message)
        .setPositiveButton(getString(R.string.ok)) { _, _ ->
          finish()
          System.exit(0)
        }
        .create().apply {
          setTitle(getString(R.string.actyx_is_stopped))
          show()
        }
    }
  }

  private fun signalShutdown() {
    Toast.makeText(this, getString(R.string.actyx_is_stopping), Toast.LENGTH_SHORT).show()
    Intent(this@MainActivity, AxNodeService::class.java).also {
      it.action = AxNodeService.STOPFOREGROUND_ACTION
      startService(it)
    }
  }

  @Suppress("DEPRECATION")
  private fun setTaskDescription() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.P) {
      val name = resources.getString(R.string.app_name)
      val icon = ContextCompat.getDrawable(this, R.drawable.actyx_icon)?.toBitmap()
      val color = ContextCompat.getColor(this, R.color.colorPrimary)
      //noinspection deprecation
      setTaskDescription(ActivityManager.TaskDescription(name, icon, color))
    }
  }
}
