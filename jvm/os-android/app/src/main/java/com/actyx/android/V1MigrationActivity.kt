package com.actyx.android

import android.Manifest
import android.annotation.SuppressLint
import android.content.Intent
import android.content.pm.ActivityInfo
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import com.actyx.android.util.Logger
import org.apache.commons.compress.archivers.tar.TarArchiveEntry
import org.apache.commons.compress.archivers.tar.TarArchiveInputStream
import org.apache.commons.compress.compressors.gzip.GzipCompressorInputStream
import java.io.BufferedInputStream
import java.io.File
import kotlin.system.exitProcess

class V1MigrationActivity : AppCompatActivity() {
  private val log = Logger()

  fun migrateData(): Result<Unit> {
    val extFilesDir = getExternalFilesDir(null)!!
    // TODO: Refactor once we support Android 11
    val v1bundle = File(Environment.getExternalStorageDirectory(), V1BundleFileName)

    if (v1bundle.exists()) {
      Toast.makeText(this, getString(R.string.migration_start), Toast.LENGTH_SHORT).show()
      log.info("Found ActyxOS v1 bundle at $v1bundle!")
      val tmp = File(cacheDir, "actyx-v1-migration")
      if (tmp.exists()) {
        tmp.deleteRecursively()
      }
      tmp.mkdirs()

      val archive =
        TarArchiveInputStream(
          GzipCompressorInputStream(
            BufferedInputStream(
              v1bundle.inputStream()
            )
          )
        )

      log.info("Unpacking v1 bundle")
      return kotlin.runCatching {
        var entry: TarArchiveEntry?
        while (archive.nextTarEntry.also { entry = it } != null) {

          if (entry?.isDirectory != false) continue

          val output = tmp.resolve(entry!!.name).also { it.parentFile?.mkdirs() }
          output.outputStream().buffered().use {
            log.info("Unpacking ${entry!!.name} to $output")
            archive.copyTo(it)
          }
        }

        log.info("Successfully unpacked archive, moving to internal data directory.")
        tmp.copyRecursively(extFilesDir)
        tmp.deleteRecursively()
        log.info("Data copied successfully.")
      }
    } else {
      return Result.failure(Throwable("$v1bundle does not exist"))
    }
  }

  fun migrateAndStart() {
    migrateData().fold({ _ ->
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        startForegroundService(Intent(this, AxNodeService::class.java))
      } else {
        startService(Intent(this, AxNodeService::class.java))
      }
      AlertDialog.Builder(this)
        .setMessage(getString(R.string.migration_successful))
        .setPositiveButton(getString(R.string.ok)) { d, _ ->
          startActivity(Intent(this, MainActivity::class.java))
          d.cancel()
        }
        .create().apply {
          setTitle(getString(R.string.migration_alert_title))
          show()
        }
    }, { t ->
      {
        AlertDialog.Builder(this)
          .setMessage("${getString(R.string.migration_failed)}\n${t.message}")
          .setPositiveButton(getString(R.string.quit)) { _, _ ->
            finish()
            exitProcess(1)
          }
          .create().apply {
            setTitle(getString(R.string.migration_alert_title))
            show()
          }
      }
    })
  }

  @SuppressLint("SourceLockedOrientationActivity")
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    log.info("onCreate: $intent")
    setContentView(R.layout.activity_v1_migration)
    if (!resources.getBoolean(R.bool.isTablet)) {
      this.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
    }

    val requestPermissionLauncher =
      registerForActivityResult(ActivityResultContracts.RequestPermission()) { isGranted: Boolean ->
        if (isGranted) {
          // yay
          log.info("permission granted")
          migrateAndStart()
        } else {
          log.error("Permission to external storage is denied")
          AlertDialog.Builder(this)
            .setMessage(getString(R.string.migration_permission_denied))
            .setPositiveButton(getString(R.string.quit)) { _, _ ->
              finish()
              exitProcess(1)
            }
            .create().apply {
              setTitle(getString(R.string.migration_alert_title))
              show()
            }
        }
      }

    if (
      ContextCompat.checkSelfPermission(
        applicationContext,
        Manifest.permission.READ_EXTERNAL_STORAGE
      ) == PackageManager.PERMISSION_GRANTED
    ) {
      migrateAndStart()
    } else {
      requestPermissionLauncher.launch(
        Manifest.permission.READ_EXTERNAL_STORAGE
      )
    }
  }

  companion object {
    val V1BundleFileName = "ActyxOS-v1-bundle.tgz"
  }
}
