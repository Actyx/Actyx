package com.actyx.os.android.util

import android.webkit.WebView
import io.reactivex.BackpressureStrategy
import io.reactivex.Flowable
import io.reactivex.Observable
import io.reactivex.Scheduler
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

object WebViewUtil {
  @Serializable
  data class WebViewMemoryStats(
    val usedJSHeapSize: Long,
    val totalJSHeapSize: Long,
    val jsHeapSizeLimit: Long
  )

  private const val script =
    // window.performance.memory doesn't stringify
    """({
          totalJSHeapSize: window.performance.memory.totalJSHeapSize,
          usedJSHeapSize: window.performance.memory.usedJSHeapSize,
          jsHeapSizeLimit: window.performance.memory.jsHeapSizeLimit
        })"""

  fun memoryStats(webView: WebView, scheduler: Scheduler): Observable<WebViewMemoryStats> =
    Flowable.create<String>(
      { emitter ->
        val worker = scheduler.createWorker()
        fun emit(mem: String) {
          worker.schedule {
            emitter.onNext(mem)
            emitter.onComplete()
          }
        }
        webView.evaluateJavascript(script, ::emit)
      },
      BackpressureStrategy.LATEST
    )
      .map { res ->
        Json.parse(WebViewMemoryStats.serializer(), res)
      }
      .subscribeOn(scheduler)
      .toObservable()
}
