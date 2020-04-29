package com.actyx.os.android.legacy.metrics

import kotlinx.serialization.*
import com.actyx.os.android.util.WebViewUtil

@Serializable
data class AndroidOrigin(var deviceId: String, var className: String) {
  val type = "android"
}

enum class MetricUnit {
  microseconds,
  bytes,
  count,
  percentage,
  mA,
  mAh,
  nWh,
  dBm,
  MHz
}

@Serializable
data class PublishMetricEvents(val data: Array<Envelope>)

@Serializable
data class Envelope(val semantics: String, val name: String, val payload: MetricsAdded)

@Serializable
data class MetricsAdded(val metrics: Array<Metric>, val type: String = "MetricsAdded")

@Serializable
@Polymorphic
sealed class Metric private constructor() {
  @Serializable
  @SerialName("numeric")
  class NumericMetric(
    val name: String,
    val value: Double,
    val unit: String,
    val timestamp: Long,
    val origin: AndroidOrigin
  ) : Metric()

  @Serializable
  @SerialName("literal")
  class LiteralMetric(
    val name: String,
    val value: String,
    val unit: String,
    val timestamp: Long,
    val origin: AndroidOrigin
  ) : Metric()

  @Serializable
  @SerialName("bool")
  class BoolMetric(
    val name: String,
    val value: Boolean,
    val unit: String,
    val timestamp: Long,
    val origin: AndroidOrigin
  ) : Metric()

  companion object {
    fun numeric(
      name: String,
      value: Double,
      unit: MetricUnit,
      timestamp: Long,
      origin: AndroidOrigin
    ): Metric {
      return NumericMetric(
        name,
        value,
        unit.name,
        timestamp,
        origin
      )
    }

    fun numeric(
      name: String,
      value: Long,
      unit: MetricUnit,
      timestamp: Long,
      origin: AndroidOrigin
    ): Metric {
      return NumericMetric(
        name,
        value.toDouble(),
        unit.name,
        timestamp,
        origin
      )
    }

    fun literal(name: String, value: String, timestamp: Long, origin: AndroidOrigin): Metric {
      return LiteralMetric(
        name,
        value,
        "literal",
        timestamp,
        origin
      )
    }

    fun bool(name: String, value: Boolean, timestamp: Long, origin: AndroidOrigin): Metric {
      return BoolMetric(
        name,
        java.lang.Boolean.valueOf(value),
        "bool",
        timestamp,
        origin
      )
    }

    fun fromDeviceInfo(i: DeviceInfo, origin: AndroidOrigin, timestamp: Long): Array<Metric> {
      val (_, uptimeMillis, disk, app, battery, network, memory, architecture) = i

      val diskMetrics = disk.map { (k, v) ->
        numeric(
          "disk_usage_$k",
          Math.round((v.total - v.free) * 100.0 / v.total),
          MetricUnit.percentage,
          timestamp,
          origin
        )
      }.toTypedArray()

      val (level, currentNow, currentAverage) = battery
      val batteryMetrics = arrayOf(
        numeric(
          "battery_level",
          level.toLong(),
          MetricUnit.percentage,
          timestamp,
          origin
        ),
        numeric(
          "battery_current_now",
          currentNow.toLong() / 1000,
          MetricUnit.mA,
          timestamp,
          origin
        ),
        numeric(
          "battery_current_avg",
          currentAverage.toLong() / 1000,
          MetricUnit.mA,
          timestamp,
          origin
        )
      )

      val networkMetrics = network?.let { (ssid, ip, rssi, freq) ->
        arrayOf(
          bool(
            "network_connected",
            true,
            timestamp,
            origin
          ),
          literal(
            "ssid",
            ssid,
            timestamp,
            origin
          ),
          literal(
            "ip",
            String.format(
              "%d.%d.%d.%d", ip and 0xff,
              ip shr 8 and 0xff, ip shr 16 and 0xff, ip shr 24 and 0xff
            ),
            timestamp,
            origin
          ),
          numeric(
            "rssi",
            rssi.toLong(),
            MetricUnit.dBm,
            timestamp,
            origin
          ),
          numeric(
            "wifi_freq",
            freq.toLong(),
            MetricUnit.MHz,
            timestamp,
            origin
          )
        )
      } ?: arrayOf(
        bool(
          "network_connected",
          false,
          timestamp,
          origin
        )
      )

      val (versionCode, versionName, firstInstall, lastUpdate) = app
      val appMetrics = arrayOf(
        literal(
          "version_code",
          versionCode.toString(),
          timestamp,
          origin
        ),
        literal(
          "version_name",
          versionName,
          timestamp,
          origin
        ),
        numeric(
          "first_install",
          firstInstall * 1000,
          MetricUnit.microseconds,
          timestamp,
          origin
        ),
        numeric(
          "last_update",
          lastUpdate * 1000,
          MetricUnit.microseconds,
          timestamp,
          origin
        )
      )

      val uptimeMetric =
        numeric(
          "uptime",
          uptimeMillis.toLong() * 1000,
          MetricUnit.microseconds,
          timestamp,
          origin
        )

      val (available, total, threshold, lowMemory) = memory
      val memoryMetrics = arrayOf(
        numeric(
          "memory_available",
          available,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        numeric(
          "memory_total",
          total,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        numeric(
          "memory_used",
          total - available,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        // How much memory used will result in Android starting to evict applications
        numeric(
          "memory_threshold",
          threshold,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        // Low water mark of memory_available. If it drops below this level, Android will start to evict applications
        bool(
          "memory_low",
          lowMemory,
          timestamp,
          origin
        )
      )

      val architectureMetric =
        literal(
          "arch",
          architecture,
          timestamp,
          origin
        )

      return diskMetrics + batteryMetrics + networkMetrics +
        appMetrics + uptimeMetric + memoryMetrics + architectureMetric
    }

    // TODO: Wire to WebViews
    fun fromMemoryStats(
      webViewMemoryStats: WebViewUtil.WebViewMemoryStats,
      origin: AndroidOrigin,
      timestamp: Long
    ): Array<Metric> {
      val (usedJSHeapSize, totalJSHeapSize, jsHeapSizeLimit) = webViewMemoryStats
      return arrayOf(
        numeric(
          "js_heap_used",
          usedJSHeapSize,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        numeric(
          "js_heap_total",
          totalJSHeapSize,
          MetricUnit.bytes,
          timestamp,
          origin
        ),
        numeric(
          "js_heap_limit",
          jsHeapSizeLimit,
          MetricUnit.bytes,
          timestamp,
          origin
        )
      )
    }
  }
}
