package com.actyx.os.android.legacy.metrics

import android.app.ActivityManager
import android.content.Context
import android.net.ConnectivityManager
import android.net.wifi.WifiManager
import android.os.BatteryManager
import android.os.Build
import android.os.Environment as E
import android.os.StatFs
import android.os.SystemClock
import java.io.File

object DeviceUtil {
  private fun getDiskInfo(dir: File): DiskInfo {
    val path = dir.absolutePath
    val stats = StatFs(path)
    return DiskInfo(path, stats.totalBytes, stats.freeBytes)
  }

  fun getDiskInfo(): Map<String, DiskInfo> {
    val dirs: Map<String, File> = mapOf(
      "data" to E.getDataDirectory(),
      "root" to E.getRootDirectory(),
      "ext" to E.getExternalStorageDirectory()
    )

    return dirs.mapValues { getDiskInfo(it.value) }
  }

  fun getAppInfo(ctx: Context): AppInfo {
    val packageInfo = ctx.packageManager.getPackageInfo(ctx.packageName, 0)
    val code = packageInfo.versionCode
    val name = packageInfo.versionName
    val firstInstallTime = packageInfo.firstInstallTime
    val lastUpdateTime = packageInfo.lastUpdateTime
    return AppInfo(code, name, firstInstallTime, lastUpdateTime)
  }

  fun getNetworkInfo(ctx: Context): NetworkInfo? {
    val cm: ConnectivityManager =
      ctx.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    val networkInfo = cm.activeNetworkInfo
    if (networkInfo != null && networkInfo.isConnected) {
      val wifiManager = ctx.applicationContext
        .getSystemService(Context.WIFI_SERVICE) as WifiManager
      val connectionInfo = wifiManager.connectionInfo
      if (connectionInfo != null) {
        return NetworkInfo(
          connectionInfo.ssid, connectionInfo.ipAddress, connectionInfo.rssi,
          connectionInfo.frequency
        )
      }
    }
    return null
  }

  fun getBatteryInfo(ctx: Context): BatteryInfo {
    val bm: BatteryManager = ctx.getSystemService(Context.BATTERY_SERVICE) as BatteryManager
    val level = bm.getIntProperty(BatteryManager.BATTERY_PROPERTY_CAPACITY)
    val currentNow = bm.getIntProperty(BatteryManager.BATTERY_PROPERTY_CURRENT_NOW)
    val currentAverage = bm.getIntProperty(BatteryManager.BATTERY_PROPERTY_CURRENT_AVERAGE)
    return BatteryInfo(level, currentNow, currentAverage)
  }

  fun getMemoryInfo(ctx: Context): DeviceMemoryInfo {
    val am = ctx.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
    val memInfo = ActivityManager.MemoryInfo()
    am.getMemoryInfo(memInfo)
    return DeviceMemoryInfo(
      memInfo.availMem, memInfo.totalMem,
      memInfo.threshold, memInfo.lowMemory
    )
  }

  fun getArchitecture(): String =
    Build.SUPPORTED_ABIS[0]

  fun getDeviceInfo(ctx: Context, serialNumber: String): DeviceInfo =
    DeviceInfo(
      serialNumber = serialNumber,
      uptimeMillis = SystemClock.elapsedRealtime(),
      disk = getDiskInfo(),
      app = getAppInfo(ctx),
      battery = getBatteryInfo(ctx),
      network = getNetworkInfo(ctx),
      memory = getMemoryInfo(ctx),
      architecture = getArchitecture()
    )
}
