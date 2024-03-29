package com.actyx.android

import android.app.*
import android.content.Intent
import android.graphics.BitmapFactory
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import com.actyx.android.node.AxNode
import com.actyx.android.node.AxNodeMessageHandler
import com.actyx.android.util.Logger

class AxNodeService : Service() {
  private val log = Logger()
  private lateinit var axNode: AxNode
  private var shutdownInitiatedByUser = false
  private val msgHandler: AxNodeMessageHandler = { code, msg ->
    log.warn("onAxNodeMessage: $code, $msg")
    stopForeground(true)
    stopSelf()
    Intent(this, MainActivity::class.java).also {
      val message = when (code) {
        AxNode.NODE_STOPPED_BY_NODE -> "Code: NODE_STOPPED_BY_NODE\n" +
          "Message: $msg\n" +
          "Please contact your IT administrator."
        AxNode.NODE_STOPPED_BY_NODE_UI -> "" // stopped by the user
        AxNode.NODE_STOPPED_BY_HOST -> "Code: NODE_STOPPED_BY_HOST\n" +
          "Message: $msg\n" +
          "Please contact your IT administrator."
        AxNode.FAILED_TO_START_NODE -> "Code: FAILED_TO_START_NODE\n" +
          "Message: $msg\n" +
          "Please contact your IT administrator."
        AxNode.ERR_PORT_COLLISION -> "Code: ERR_PORT_COLLISION\n" +
          "Message: $msg\n" +
          "Please contact your IT administrator."
        else -> "Code: $code\n" +
          "Message: $msg\n" +
          "Please contact your IT administrator."
      }
      it.putExtra(AXNODE_MESSAGE, message)
      it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      startActivity(it)
    }
  }

  override fun onCreate() {
    axNode = AxNode(this, msgHandler)
    log.info("Initialization of native platform lib done")
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    log.info("onStartCommand, intent: $intent")
    if (intent?.action.equals(STOPFOREGROUND_ACTION)) {
      log.info("Received Stop Foreground Intent")
      axNode.shutdownByUser()
      shutdownInitiatedByUser = true
    }
    startForeground(ONGOING_NOTIFICATION_ID, setupNotification())
    return START_STICKY
  }

  override fun onBind(intent: Intent?): IBinder? {
    return null
  }

  override fun onLowMemory() {
    log.warn("onLowMemory")
    super.onLowMemory()
  }

  override fun onDestroy() {
    log.info("onDestroy")
    if (!shutdownInitiatedByUser) {
      // Try to shutdown gracefully ax node
      axNode.shutdownBySystem()
    }
    super.onDestroy()
  }

  private fun setupNotification(): Notification {
    val intent = Intent(this, MainActivity::class.java)
    val pendingIntent = PendingIntent.getActivity(this, 0, intent, 0)
    val contextText = getString(R.string.actyx_is_running)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        NOTIFICATION_CHANNEL_ID,
        getString(R.string.background_services_channel_name),
        NotificationManager.IMPORTANCE_LOW
      )
      channel.description = getString(R.string.background_services_channel_description)
      getSystemService(NotificationManager::class.java)?.createNotificationChannel(channel)
    }

    val requestQuitIntent = Intent(this, MainActivity::class.java).apply {
      action = REQUEST_QUIT
    }

    return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
      .setOngoing(true)
      .setContentTitle(contextText)
      .setSmallIcon(R.drawable.actyx_icon)
      .setColor(ContextCompat.getColor(this, R.color.colorPrimary))
      .setLargeIcon(BitmapFactory.decodeResource(resources, R.drawable.actyx_icon))
      .setCategory(Notification.CATEGORY_SERVICE)
      .setContentIntent(pendingIntent)
      .setPriority(NotificationCompat.PRIORITY_LOW)
      .addAction(
        0, getString(R.string.quit),
        PendingIntent.getActivity(this, 0, requestQuitIntent, 0)
      )
      .build()
  }

  companion object {
    const val NOTIFICATION_CHANNEL_ID = "com.actyx.android"
    const val ONGOING_NOTIFICATION_ID = 7
    const val STOPFOREGROUND_ACTION = "com.actyx.android.STOPFOREGROUND_ACTION"
    const val AXNODE_MESSAGE = "com.actyx.android.AXNODE_MESSAGE"
    const val REQUEST_QUIT = "com.actyx.android.REQUEST_QUIT"
  }
}
