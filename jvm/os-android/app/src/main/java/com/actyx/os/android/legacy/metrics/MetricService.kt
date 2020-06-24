package com.actyx.os.android.legacy.metrics

import android.content.Context
import com.actyx.os.android.model.ActyxOsSettings
import com.actyx.os.android.service.Service
import com.actyx.os.android.util.DeviceSerialNr
import com.actyx.os.android.util.Logger
import io.reactivex.Observable
import io.reactivex.Single
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonConfiguration
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.util.*
import java.util.concurrent.TimeUnit

class MetricService(private val ctx: Context) : Service {

  val log = Logger()

  override fun invoke(settings: ActyxOsSettings): Single<Unit> {
    val deviceId = DeviceSerialNr.get(ctx)
    val origin = AndroidOrigin(deviceId, this.javaClass.name)
    return Observable.interval(1, 15, TimeUnit.MINUTES)
      .concatMap { _ ->
        val timestamp = Date().time * 1000
        val deviceInfo = DeviceUtil.getDeviceInfo(ctx, deviceId)
        val metrics = Metric.fromDeviceInfo(deviceInfo, origin, timestamp)
        val publishMetrics = PublishMetricEvents(
          arrayOf(Envelope(semantics, semantics, MetricsAdded(metrics)))
        )
        val json = serializer.stringify(PublishMetricEvents.serializer(), publishMetrics)
        val body = json.toRequestBody(jsonContentType)
        val request = Request.Builder().url(endpoint).post(body).build()

        log.debug("Sending {} device metrics to {}.", metrics.size, endpoint)

        Observable
          .fromCallable { httpClient.newCall(request).execute() }
          .map { response ->
            if (response.code != 201) {
              throw RuntimeException("Bad response: ${response.code} ${response.body}")
            }
            log.info("Successfully sent metrics.")
          }
          .onErrorResumeNext { e: Throwable ->
            log.error("Error sending metrics: ${e.message}", e)
            Observable.empty<Unit>()
          }
      }
      .last(Unit)
  }

  companion object {
    private val httpClient = OkHttpClient()
    private val serializer =
      Json(configuration = JsonConfiguration.Default.copy(useArrayPolymorphism = false))
    val jsonContentType = ("application/json; charset=utf-8").toMediaType()
    private const val endpoint = "http://localhost:4454/api/v1/events/publish"
    private const val semantics = "edge.ax.sf.metrics"
  }
}
