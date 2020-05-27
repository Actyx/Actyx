package com.actyx.os.android.fragment

import android.net.http.SslError
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.webkit.*
import androidx.fragment.app.Fragment
import kotlinx.android.synthetic.main.fragment_webview.view.*
import kotlinx.serialization.Serializable
import kotlinx.serialization.SerializationStrategy
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonConfiguration

import com.actyx.os.android.R
import com.actyx.os.android.util.Logger

@Serializable
data class SwipedCardData(val cardId: String, val provenance: String)

@Serializable
data class ScannedCodeData(val code: String, val provenance: String)

class WebViewFragment : Fragment() {
  private val log = Logger()
  private var url: String? = null
  private var settings: String? = null

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View {
    val view = inflater.inflate(R.layout.fragment_webview, container, false)
    view.webview.apply {
      WebView.setWebContentsDebuggingEnabled(true)
      settings.javaScriptEnabled = true
      settings.domStorageEnabled = true
      // No caching, as this influences our ability to update an app.
      settings.cacheMode = WebSettings.LOAD_NO_CACHE
      webViewClient = AXWebViewClient()
      webChromeClient = object : WebChromeClient() {
        override fun onPermissionRequest(request: PermissionRequest) {
          request.grant(request.resources)
        }
      }
    }

    return view
  }

  override fun onActivityCreated(savedInstanceState: Bundle?) {
    super.onActivityCreated(savedInstanceState)
    url?.let { x -> settings?.let { y -> loadWebappWithSettings(x, y) } }
  }

  fun loadWebappWithSettings(url: String, settings: String) {
    this.url = url
    this.settings = settings
    view?.let {
      log.debug("loading url")
      it.webview.addJavascriptInterface(JsInterface(settings), "ax")
      it.webview.loadUrl(url)
    } ?: log.debug("View is undefined in update()")
  }

  fun dispatchCardSwipedCustomEvent(data: SwipedCardData) {
    log.info("card swiped: {}", data)
    dispatchCustomEvent(CARD_SCANNED, SwipedCardData.serializer(), data)
  }

  fun dispatchCodeScannedCustomEvent(data: ScannedCodeData) {
    log.info("code scanned: {}", data)
    dispatchCustomEvent(CODE_SCANNED, ScannedCodeData.serializer(), data)
  }

  private fun <T> dispatchCustomEvent(
    eventType: String,
    serializer: SerializationStrategy<T>,
    data: T
  ) {
    view?.let {
      it.webview.evaluateJavascript(mkDispatchCustomEventCode(eventType, serializer, data)) {}
    } ?: log.debug("View is undefined in dispatchCustomEvent()")
  }

  // We can't use an anonymous class here, as this requires at least Java 8,
  // when used w/ `addJavascriptInterface`.
  private inner class JsInterface(val cfg: String) {

    @JavascriptInterface
    fun appSettings() = cfg
  }

  private inner class AXWebViewClient : WebViewClient() {
    override fun onReceivedSslError(
      view: WebView,
      handler: SslErrorHandler,
      error: SslError
    ) {
      handler.proceed()
    }

    override fun onPageFinished(view: WebView, url: String) {
      super.onPageFinished(view, url)
      view.visibility = View.VISIBLE
    }
  }

  companion object {
    private const val CARD_SCANNED = "com.actyx.os.hardware.event.CARD_SCANNED"
    private const val CODE_SCANNED = "com.actyx.os.hardware.event.CODE_SCANNED"
    private val json = Json(JsonConfiguration.Stable)
    private fun <T> mkDispatchCustomEventCode(
      eventType: String,
      serializer: SerializationStrategy<T>,
      data: T
    ): String {
      val jsonData = json.stringify(serializer, data)
      // NB! `detail` prop has to be used to pass custom data via CustomEvent
      return "window.dispatchEvent(new CustomEvent('$eventType', { detail: $jsonData }))"
    }
  }
}
