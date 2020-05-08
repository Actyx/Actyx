package com.actyx.os.android.activity

import android.app.ActivityManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.BitmapFactory
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.fragment.app.Fragment
import com.actyx.os.android.AppInfo
import com.actyx.os.android.R
import com.actyx.os.android.fragment.MessageFragment
import com.actyx.os.android.fragment.SwipedCardData
import com.actyx.os.android.fragment.WebViewFragment
import com.actyx.os.android.legacy.nfc.NfcIntentReceiverActivity
import com.actyx.os.android.legacy.usb.BaltechReaderService
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.service.IBackgroundServices
import com.actyx.os.android.util.WebappTracker
import io.reactivex.disposables.Disposable
import io.reactivex.schedulers.Schedulers
import io.reactivex.subjects.BehaviorSubject

sealed class AppSettings {
  object Initializing : AppSettings()
  object MissingOrInvalid : AppSettings()
  class Valid(val settings: String, val appInfo: AppInfo) : AppSettings()
}

/**
 * An example full-screen activity that shows and hides the system UI (i.e.
 * status bar and navigation/system bar) with user interaction.
 */
class WebappActivity : BaseActivity() {

  private var backgroundServices: IBackgroundServices? = null
  private lateinit var appId: String
  private val settingsSubject = BehaviorSubject.create<AppSettings>()
  private lateinit var settingsDisposable: Disposable
  private lateinit var webView: WebViewFragment
  private lateinit var message: MessageFragment
  private lateinit var tracker: WebappTracker

  private val settingsUpdatedReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      backgroundServices?.let {
        val scope = intent.getStringExtra(BackgroundServices.EXTRA_SETTINGS_SCOPE)
        if (scope?.startsWith(appId) == true || scope == BackgroundServices.ROOT_SCOPE) {
          settingsSubject.onNext(getAppSettings(appId, it))
        }
      }
    }
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContentView(R.layout.activity_webapp)
    webView = WebViewFragment()
    message = MessageFragment()

    appId = intent.getStringExtra(EXTRA_SHORTCUT_APP_ID)!!
    settingsDisposable =
      settingsSubject
        .startWith(AppSettings.Initializing)
        .distinctUntilChanged()
        .subscribeOn(Schedulers.io())
        .subscribe(::renderUi)
    tracker = WebappTracker(this, lifecycle, appId)
  }

  private val stopReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      val appId = intent.getStringExtra(BackgroundServices.EXTRA_APP_ID)
      if (appId == this@WebappActivity.appId) {
        finishAndRemoveTask()
      }
    }
  }

  override fun onStart() {
    super.onStart()
    registerReceiver(
      settingsUpdatedReceiver,
      IntentFilter(BackgroundServices.ACTION_SETTINGS_UPDATED)
    )
    registerReceiver(
      stopReceiver,
      IntentFilter(BackgroundServices.ACTION_APP_STOP_REQUESTED)
    )
    registerReceiver(
      cardScannedReceiver,
      IntentFilter(BaltechReaderService.ACTION_CARD_SCANNED)
    )
    registerReceiver(
      nfcCardScannedReceiver,
      IntentFilter(NfcIntentReceiverActivity.NFCA_TAG_ID_SCANNED)
    )
  }

  override fun onResume() {
    super.onResume()
    window.decorView.apply {
      systemUiVisibility =
        View.SYSTEM_UI_FLAG_LOW_PROFILE or // dimmed top/bottom bars
          View.SYSTEM_UI_FLAG_FULLSCREEN or // hide top status bar
          View.SYSTEM_UI_FLAG_LAYOUT_STABLE or // overlay bars when shown
          View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY or // don't clear other flags on user interaction
          View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION or // overlay bars when shown
          View.SYSTEM_UI_FLAG_HIDE_NAVIGATION // hide bottom nav bar
    }
  }

  override fun onStop() {
    super.onStop()
    unregisterReceiver(settingsUpdatedReceiver)
    unregisterReceiver(stopReceiver)
    unregisterReceiver(cardScannedReceiver)
    unregisterReceiver(nfcCardScannedReceiver)
  }

  override fun onDestroy() {
    super.onDestroy()
    settingsDisposable.dispose()
    tracker.close()
  }

  override fun onBackPressed() {
    // The default impl destroys the association between the start intent data and this instance of
    // the activity so that a new (possibly duplicated) instance of the activity is created
    // whenever a new start intent is sent.
    moveTaskToBack(true)
  }

  override fun onBackgroundServicesConnected(backgroundServices: IBackgroundServices) {
    this.backgroundServices = backgroundServices
    val appInfo = backgroundServices.getAppInfo(appId)
    setTaskDescription(
      ActivityManager.TaskDescription(
        appInfo.name,
        BitmapFactory.decodeFile(appInfo.iconPath)
      )
    )

    settingsSubject.onNext(getAppSettings(appId, backgroundServices))
  }

  override fun onBackgroundServicesDisconnected() {
    this.backgroundServices = null
  }

  private val cardScannedReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      intent.getStringExtra(BaltechReaderService.EXTRA_SNR)?.let {
        toast("Scanned card (external) $it")
        webView.dispatchCardSwipedCustomEvent(SwipedCardData(it, "baltech card reader"))
      }
    }
  }

  private val nfcCardScannedReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      intent.getStringExtra(NfcIntentReceiverActivity.EXTRA_TAG_ID)?.let {
        toast("Scanned card (internal) $it")
        webView.dispatchCardSwipedCustomEvent(SwipedCardData(it, "internal nfc"))
      }
    }
  }

  private fun toast(msg: String) {
    Toast.makeText(applicationContext, msg, Toast.LENGTH_SHORT).show()
  }

  private fun renderUi(appSettings: AppSettings) {
    log.debug("renderUi: {}", appSettings)
    val fragment: Fragment = when (appSettings) {
      is AppSettings.Initializing -> {
        message.update(R.string.initializing)
        message
      }
      is AppSettings.MissingOrInvalid -> {
        message.update(R.string.invalid_settings_msg)
        message
      }
      is AppSettings.Valid -> {
        webView.loadWebappWithSettings(appSettings.appInfo.uri.toString(), appSettings.settings)
        webView
      }
    }

    supportFragmentManager
      .beginTransaction()
      .replace(R.id.fragment_container, fragment)
      .commit()
  }

  companion object {
    const val EXTRA_SHORTCUT_APP_ID = "com.actyx.intent.extra.shortcut.APP_ID"

    private fun getAppSettings(
      appId: String,
      backgroundServices: IBackgroundServices
    ): AppSettings {
      /**
       * This is nullable. Null indicates an error. This can be interpreted here as a missing or
       * invalid configuration.
       */
      return backgroundServices.getSettings(appId)?.let {
        val appInfo = backgroundServices.getAppInfo(appId)
        AppSettings.Valid(it, appInfo)
      }
        ?: AppSettings.MissingOrInvalid
    }
  }
}
