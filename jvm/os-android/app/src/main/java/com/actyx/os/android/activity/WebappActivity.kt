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
import com.actyx.os.android.util.collectNonNull
import io.reactivex.Observable
import io.reactivex.ObservableSource
import io.reactivex.android.schedulers.AndroidSchedulers
import io.reactivex.disposables.Disposable
import io.reactivex.functions.Function
import io.reactivex.schedulers.Schedulers
import io.reactivex.subjects.BehaviorSubject
import java.util.concurrent.TimeUnit

sealed class AppState {
  object WaitingForStartCommand : AppState()
  object MissingOrInvalidSettings : AppState()
  class Running(val settings: String, val appInfo: AppInfo) : AppState()
}

object StartAppCommand

/**
 * An example full-screen activity that shows and hides the system UI (i.e.
 * status bar and navigation/system bar) with user interaction.
 */
class WebappActivity : BaseActivity() {

  private var backgroundServices: IBackgroundServices? = null
  private lateinit var appId: String
  private lateinit var webView: WebViewFragment
  private lateinit var message: MessageFragment
  private lateinit var tracker: WebappTracker
  private val state = BehaviorSubject.create<StartAppCommand>()
  private lateinit var stateDisposable: Disposable

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    setContentView(R.layout.activity_webapp)
    webView = WebViewFragment()
    message = MessageFragment()

    appId = intent.getStringExtra(EXTRA_SHORTCUT_APP_ID)!!
    stateDisposable =
      state
        .collectNonNull { backgroundServices?.let { getAppSettings(appId, it) } }
        // If we don't get a StartAppCommand from the node, indicate an error:
        .timeout(
          // Timeout only for first item
          Observable.timer(3, TimeUnit.SECONDS).observeOn(AndroidSchedulers.mainThread()),
          Function<AppState, ObservableSource<AppState>> { Observable.never() }
        )
        // Timeout above throws a TimeoutException, which we catch here
        .onErrorReturn { AppState.MissingOrInvalidSettings }
        .startWith(AppState.WaitingForStartCommand)
        .distinctUntilChanged()
        .doOnNext {
          when (it) {
            is AppState.Running ->
              backgroundServices?.onAppStarted(appId)
            else -> {} // ignore
          }
        }
        .subscribeOn(Schedulers.io())
        .observeOn(AndroidSchedulers.mainThread())
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

  private val startReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      val appId = intent.getStringExtra(BackgroundServices.EXTRA_APP_ID)
      if (appId == this@WebappActivity.appId) {
        state.onNext(StartAppCommand)
      }
    }
  }

  override fun onStart() {
    super.onStart()
    registerReceiver(
      stopReceiver,
      IntentFilter(BackgroundServices.ACTION_APP_STOP_REQUESTED)
    )
    registerReceiver(
      startReceiver,
      IntentFilter(BackgroundServices.ACTION_APP_START_REQUESTED)
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
    unregisterReceiver(startReceiver)
    unregisterReceiver(stopReceiver)
    unregisterReceiver(cardScannedReceiver)
    unregisterReceiver(nfcCardScannedReceiver)
  }

  override fun onDestroy() {
    super.onDestroy()
    stateDisposable.dispose()
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

  private fun renderUi(appState: AppState) {
    log.debug("renderUi: {}", appState)
    val fragment: Fragment = when (appState) {
      is AppState.WaitingForStartCommand -> {
        message.update(R.string.initializing)
        message
      }
      // FIXME: How to determine this? E.g. user wants to start a currently misconfigured app
      // --> startApp command will never come.. Maybe timeout here?
      is AppState.MissingOrInvalidSettings -> {
        message.update(R.string.invalid_settings_msg)
        message
      }
      is AppState.Running -> {
        webView.loadWebappWithSettings(appState.appInfo.uri.toString(), appState.settings)
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
    ): AppState {
      /**
       * This is nullable. Null indicates an error. This can be interpreted here as a missing or
       * invalid configuration.
       */
      return backgroundServices.getSettings(appId)?.let {
        val appInfo = backgroundServices.getAppInfo(appId)
        AppState.Running(it, appInfo)
      }
        ?: AppState.MissingOrInvalidSettings
    }
  }
}
