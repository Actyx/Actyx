package com.actyx.os.android.service

import android.app.*
import android.app.Service
import android.content.Intent
import android.content.res.Configuration
import android.graphics.BitmapFactory
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import arrow.core.getOrElse
import arrow.core.orNull
import com.actyx.os.android.AppRepository
import com.actyx.os.android.R
import com.actyx.os.android.activity.MainActivity
import com.actyx.os.android.legacy.metrics.MetricService
import com.actyx.os.android.legacy.usb.BaltechReaderService
import com.actyx.os.android.model.ActyxOsSettings
import com.actyx.os.android.osnode.AxNode
import com.actyx.os.android.osnode.model.NodeFfi.ToAndroid
import com.actyx.os.android.util.Logger
import com.actyx.os.android.util.lens
import com.actyx.os.android.util.retryAlways
import io.reactivex.Observable
import io.reactivex.disposables.CompositeDisposable
import io.reactivex.disposables.Disposable
import io.reactivex.schedulers.Schedulers
import io.reactivex.subjects.PublishSubject
import kotlinx.serialization.json.*

typealias Scope = String

class BackgroundServices : Service() {
  data class SettingsUpdate(val scope: Scope, val settingsJson: String)

  private val log = Logger()

  private lateinit var appRepository: AppRepository
  private lateinit var axNode: AxNode
  private lateinit var disposables: Disposable

  private val settingsUpdates = PublishSubject.create<SettingsUpdate>()
  private val appUpdates = PublishSubject.create<String>()

  private val binder = object : IBackgroundServices.Stub() {
    override fun getApps() = appRepository.appInfoList().toMutableList()

    override fun getAppInfo(appId: String) = appRepository.appInfo(appId)

    override fun getSettings(scope: Scope) = axNode.getSettings(scope).orNull()

    override fun onAppStarted(appId: String) = axNode.notifyAppStarted(appId)

    override fun onAppStopped(appId: String) = axNode.notifyAppStopped(appId)

    override fun onAppEnabled(appId: String) = axNode.notifyAppEnabled(appId)

    override fun onAppDisabled(appId: String) = axNode.notifyAppDisabled(appId)
  }

  private fun handler(msg: ToAndroid) {
    when (msg.payloadCase) {
      ToAndroid.PayloadCase.SETTINGSUPDATED ->
        settingsUpdates.onNext(
          SettingsUpdate(
            msg.settingsUpdated.changedScope,
            msg.settingsUpdated.rootSettingsJson
          )
        )
      ToAndroid.PayloadCase.APPDEPLOYED ->
        appUpdates.onNext(msg.appDeployed.appId)
      ToAndroid.PayloadCase.APPUNDEPLOYED ->
        appUpdates.onNext(msg.appUndeployed.appId)
      ToAndroid.PayloadCase.STARTAPP ->
        appRepository.startApp(msg.startApp.appId)
      ToAndroid.PayloadCase.STOPAPP ->
        appRepository.stopApp(msg.stopApp.appId)
      ToAndroid.PayloadCase.NODESTATECHANGED ->
        setupNotificationAndService(msg.nodeStateChanged.state)
    }
  }

  fun setupNotificationAndService(nodeState: ToAndroid.NodeStateChanged.State) {
    stopForeground(true)

    setupNotification(nodeState).also { n ->
      log.debug("BackgroundServices.startForeground($n)")
      startForeground(1, n)
    }
  }

  override fun onBind(intent: Intent): IBinder? {
    log.debug("onBind, intent: $intent")

    // FIXME remove
    val initialSettings =
      axNode.getSettings(SYSTEM_SETTINGS_SCOPE)
        .fold(
          {
            log.warn(
              "Error getting initial system-scope settings: \"[{}]\". " +
                "Note: This is expected for an unconfigured node.", it
            )
            Observable.empty<String>()
          },
          { Observable.just(it) }
        )
        // `getSettings` returns the settings object scoped to the queried scope
        // thus, we need to fabricate a fitting object here
        .map {
          jsonSerialization.stringify(
            JsonElementSerializer, JsonObject(
              mapOf(SYSTEM_SETTINGS_SCOPE to jsonSerialization.parseJson(it))
            )
          )
        }
        .map { SettingsUpdate(SYSTEM_SETTINGS_SCOPE, it) }

    val servicesDisposable =
      Observable.concat(initialSettings, settingsUpdates)
        .filter { (scope) -> scope.startsWith(SYSTEM_SETTINGS_SCOPE) }
        .map { (_, settingsJson) ->
          val settings = jsonSerialization.parseJson(settingsJson)
          settings.lens(listOf(SYSTEM_SETTINGS_SCOPE)).getOrElse {
            throw RuntimeException(
              "Couldn't get scope '$SYSTEM_SETTINGS_SCOPE' within '$settingsJson'"
            )
          }
        }
        .distinctUntilChanged()
        .map(::toSettings)
        .onErrorResumeNext { it: Throwable ->
          log.error("Error getting settings: ${it.message}", it)
          Observable.empty()
        }
        .switchMap(::runServices)
        .subscribeOn(Schedulers.io())
        .subscribe()

    val appUpdatesDisposable =
      appUpdates
        .subscribeOn(Schedulers.io())
        .subscribe { sendBroadcast(Intent(ACTION_APP_LIST_UPDATED)) }

    val settingsUpdatesDisposable =
      settingsUpdates
        .subscribeOn(Schedulers.io())
        .subscribe { (scope, _) ->
          sendBroadcast(Intent(ACTION_SETTINGS_UPDATED).apply {
            putExtra(EXTRA_SETTINGS_SCOPE, scope)
          })
        }

    val restServiceDesposible = RestService(appRepository).invoke().subscribe()

    disposables = CompositeDisposable(
      servicesDisposable,
      appUpdatesDisposable,
      settingsUpdatesDisposable,
      restServiceDesposible
    )

    return binder
  }

  override fun onUnbind(intent: Intent?): Boolean {
    log.debug("onUnbind, intent: $intent")
    disposables.dispose()
    return super.onUnbind(intent)
  }

  override fun onCreate() {
    log.debug("onCreate")
    super.onCreate()
    setupNotificationAndService(ToAndroid.NodeStateChanged.State.MISCONFIGURED)
    axNode = AxNode(this, ::handler)

    val extFilesDir = applicationContext.getExternalFilesDir(null)!!
    // TODO check if null
    appRepository = AppRepository(extFilesDir, this)
  }

  private fun setupNotification(nodeState: ToAndroid.NodeStateChanged.State): Notification {
    val intent = Intent(this, MainActivity::class.java)
    val pendingIntent = PendingIntent.getActivity(this, 0, intent, 0)
    val contextText = when (nodeState) {
      ToAndroid.NodeStateChanged.State.CONFIGURED ->
        getString(R.string.background_services_operational_text)
      else -> getString(R.string.background_services_misconfigured_text)
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      val channel = NotificationChannel(
        NOTIFICATION_CHANNEL_ID,
        getString(R.string.background_services_channel_name),
        NotificationManager.IMPORTANCE_LOW
      ).apply {
        description = getString(R.string.background_services_channel_description)
      }
      getSystemService(NotificationManager::class.java)
        ?.createNotificationChannel(channel)

      return Notification.Builder(this, NOTIFICATION_CHANNEL_ID)
        .setOngoing(true)
        .setContentTitle(getString(R.string.background_services_name))
        .setContentText(contextText)
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setAutoCancel(true)
        .setContentIntent(pendingIntent)
        .build()
    } else {
      return NotificationCompat.Builder(this, NOTIFICATION_CHANNEL_ID)
        .setContentTitle(getString(R.string.background_services_name))
        .setContentText(contextText)
        .setSmallIcon(android.R.drawable.stat_notify_sync)
        .setLargeIcon(BitmapFactory.decodeResource(resources, R.mipmap.ic_launcher))
        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setAutoCancel(true)
        .setContentIntent(pendingIntent)
        .build()
    }
  }

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    log.debug("onStartCommand, intent: $intent, flags: $flags, startId: $startId")
    return super.onStartCommand(intent, flags, startId)
  }

  private fun runServices(settings: ActyxOsSettings): Observable<Unit> {
    return Observable.merge(
      mapOf(
        "baltech" to BaltechReaderService(this),
        "metric" to MetricService(this),
        "ipfs" to IpfsService(this)
      )
        .map { (name, run) ->
          run(settings)
            .retryAlways { err ->
              log.error("Error running service $name: ${err.message}. Restarting...", err)
            }
            .doOnSuccess { log.info("Service $name stopped. Restarting...") }
            .repeat()
            .toObservable()
            .subscribeOn(Schedulers.io())
        })
  }

  override fun onDestroy() {
    log.debug("onDestroy")
    super.onDestroy()
  }

  override fun onLowMemory() {
    log.warn("onLowMemory")
    super.onLowMemory()
  }

  override fun onRebind(intent: Intent?) {
    log.debug("onRebind")
    super.onRebind(intent)
  }

  override fun onTaskRemoved(rootIntent: Intent?) {
    super.onTaskRemoved(rootIntent)
    log.debug("onTaskRemoved, rootIntent: $rootIntent")
  }

  override fun onConfigurationChanged(newConfig: Configuration) {
    log.debug("onConfigurationChanged, newConfig: $newConfig")
    super.onConfigurationChanged(newConfig)
  }

  companion object {
    const val NOTIFICATION_CHANNEL_ID = "com.actyx.os.android"

    const val ACTION_APP_LIST_UPDATED = "com.actyx.os.apps.APP_LIST_UPDATED"
    const val ACTION_SETTINGS_UPDATED = "com.actyx.os.settings.SETTINGS_UPDATED"
    const val EXTRA_SETTINGS_SCOPE = "com.actyx.intent.extra.settings.SCOPE"
    // Intent to start the actual WebApp inside WebAppActivity
    const val ACTION_APP_STOP_REQUESTED = "com.actyx.os.apps.APP_STOP_REQUESTED"
    const val ACTION_APP_START_REQUESTED = "com.actyx.os.apps.APP_START_REQUESTED"
    const val EXTRA_APP_ID = "com.actyx.os.intent.extra.app.APP_ID"

    private fun toSettings(json: JsonElement): ActyxOsSettings =
      jsonSerialization.fromJson(ActyxOsSettings.serializer(), json)

    // TODO move somewhere else?
    val jsonSerialization = Json(JsonConfiguration.Default.copy(strictMode = false))

    const val ROOT_SCOPE = "."
    const val HIERARCHY_SEPARATOR = "/"
    const val SYSTEM_SETTINGS_SCOPE = "com.actyx.os"
  }
}
