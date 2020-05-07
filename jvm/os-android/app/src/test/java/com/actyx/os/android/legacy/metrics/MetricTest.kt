package com.actyx.os.android.legacy.metrics

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonConfiguration
import kotlinx.serialization.list
import org.junit.Assert.assertEquals
import org.junit.Test

class MetricTest {

  private val timestamp = 1554810304210000
  private val serialNr = "BR123456"
  private val uptimeMillis = 100000
  private val disks = mapOf(
    "data" to DiskInfo(
      "/data",
      101,
      33
    )
  )
  private val appInfo = AppInfo(
    35, "1.9.8",
    timestamp - 200000, timestamp - 100000
  )
  private val batteryInfo =
    BatteryInfo(99, 586000, 200000)
  private val networkInfo = NetworkInfo(
    "Actyx",
    323223577,
    -66,
    55000000
  )
  private val memoryInfo =
    DeviceMemoryInfo(1234, 4242, 4200, false)
  private val devInfo = DeviceInfo(
    serialNr, uptimeMillis, disks, appInfo,
    batteryInfo, networkInfo, memoryInfo, "x86"
  )
  private val origin = AndroidOrigin(
    serialNr,
    this.javaClass.name
  )

  @Test
  fun fromDeviceInfo() {
    val expected = arrayOf(
      Metric.numeric("disk_usage_data", 67, MetricUnit.percentage, timestamp, origin),
      Metric.numeric("battery_level", 99, MetricUnit.percentage, timestamp, origin),
      Metric.numeric("battery_current_now", 586, MetricUnit.mA, timestamp, origin),
      Metric.numeric("battery_current_avg", 200, MetricUnit.mA, timestamp, origin),
      Metric.bool("network_connected", true, timestamp, origin),
      Metric.literal("ssid", "Actyx", timestamp, origin),
      Metric.literal("ip", "25.0.68.19", timestamp, origin),
      Metric.numeric("rssi", -66, MetricUnit.dBm, timestamp, origin),
      Metric.numeric("wifi_freq", 55000000, MetricUnit.MHz, timestamp, origin),
      Metric.literal("version_code", "35", timestamp, origin),
      Metric.literal("version_name", "1.9.8", timestamp, origin),
      Metric.numeric(
        "first_install",
        1554810304010000000, MetricUnit.microseconds, timestamp, origin
      ),
      Metric.numeric(
        "last_update", 1554810304110000000,
        MetricUnit.microseconds, timestamp, origin
      ),
      Metric.numeric("uptime", 100000000, MetricUnit.microseconds, timestamp, origin),
      Metric.numeric("memory_available", 1234, MetricUnit.bytes, timestamp, origin),
      Metric.numeric("memory_total", 4242, MetricUnit.bytes, timestamp, origin),
      Metric.numeric("memory_used", 4242 - 1234, MetricUnit.bytes, timestamp, origin),
      Metric.numeric("memory_threshold", 4200, MetricUnit.bytes, timestamp, origin),
      Metric.bool("memory_low", false, timestamp, origin),
      Metric.literal("arch", "x86", timestamp, origin)
    )
    val json = Json(configuration = JsonConfiguration(useArrayPolymorphism = false))
    val exp = json.stringify(Metric.serializer().list, expected.toList())
    val actual = json.stringify(
      Metric.serializer().list, Metric.fromDeviceInfo(
        devInfo, origin,
        timestamp
      ).toList()
    )
    assertEquals(exp, actual)
  }
}
