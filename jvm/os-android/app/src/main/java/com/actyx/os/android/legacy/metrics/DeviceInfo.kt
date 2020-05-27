package com.actyx.os.android.legacy.metrics

data class DiskInfo(
  val path: String,
  val total: Long,
  val free: Long
)

data class AppInfo(
  val versionCode: Int,
  val versionName: String,
  val firstInstallTime: Long,
  val lastUpdateTime: Long
)

data class BatteryInfo(
  val level: Int,
  val currentNow: Int,
  val currentAverage: Int
)

data class NetworkInfo(
  val ssid: String,
  val ipAddress: Int,
  val rssi: Int,
  val fequency: Int
)

data class DeviceMemoryInfo(
  val available: Long,
  val total: Long,
  val threshold: Long,
  val lowMemory: Boolean
)

data class DeviceInfo(
  val serialNumber: String,
  val uptimeMillis: Number,
  val disk: Map<String, DiskInfo>,
  val app: AppInfo,
  val battery: BatteryInfo,
  val network: NetworkInfo?,
  val memory: DeviceMemoryInfo,
  val architecture: String
)
