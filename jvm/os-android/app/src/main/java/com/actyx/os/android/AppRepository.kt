package com.actyx.os.android

import android.app.KeyguardManager
import android.content.Context
import android.content.Intent
import android.content.Intent.FLAG_ACTIVITY_NEW_TASK
import android.net.Uri
import android.os.PowerManager
import arrow.core.Either
import arrow.core.Option
import com.actyx.os.android.activity.MainActivity
import com.actyx.os.android.activity.WebappActivity
import com.actyx.os.android.api.RestServer
import com.actyx.os.android.service.BackgroundServices
import com.actyx.os.android.util.Descriptor
import com.actyx.os.android.util.Logger
import io.reactivex.subjects.BehaviorSubject
import java.io.File
import java.io.FileInputStream
import java.io.InputStream

/**
 * Repository of deployed ActyxOS Webapps.
 *
 * Structure:
 *    root
 *    └─ {appId}
 *       ├─ current // text file that contains the latest version
 *       └─ {version}
 *          ├─ ax-descriptor.yml
 *          └─ {extracted app archive content}
 */
class AppRepository(extFilesDir: File, val ctx: Context) {

  private val log = Logger()

  private val tempDir = File(extFilesDir, "tmp")
  private val baseDir = File(extFilesDir, "apps")

  private val appsSubject = BehaviorSubject.create<List<AppInfo>>()

  init {
    if (!baseDir.exists()) baseDir.mkdir()
    if (!tempDir.exists()) tempDir.mkdir()
    appsSubject.onNext(appInfoList())
  }

  private fun appInfo(descriptor: Descriptor): AppInfo =
    AppInfo(
      descriptor.id,
      descriptor.version,
      descriptor.displayName,
      iconFile(descriptor)?.absolutePath,
      appUrl(descriptor),
      loadSettingsSchema(descriptor)
    )

  fun appInfo(appId: String): AppInfo? =
    currentAppDir(appId)?.let { current ->
      appInfo(descriptor(current))
    }

  /**
   * returns a list of all currently deployed apps
   */
  fun appInfoList(): List<AppInfo> =
    baseDir.listFiles { f -> f.isDirectory }
      ?.map { it.name }
      ?.mapNotNull(::appInfo)
      ?: listOf()

  /**
   * loads the specified resource from the dist directory of the latest version of the
   * specified appid
   */
  fun getAppResourceAsStream(appId: String, resourcePath: String): Pair<InputStream, Long> =
    currentDistDir(appId)?.let { distDir ->
      File(distDir, resourcePath).let {
        if (it.isFile)
          Pair(FileInputStream(it), it.length())
        else
          throw ResourceNotFoundException(
            "Resource for app \"$appId\" not found: \"$resourcePath\""
          )
      }
    } ?: throw ResourceNotFoundException("App not found: \"$appId\"")

  /**
   * gets the app icon path referenced in the descriptor within the repo
   */
  private fun iconFile(descriptor: Descriptor): File? =
    descriptor.icon?.let { File(currentAppDir(descriptor.id), it) }

  /**
   * creates the url under which the app is served
   */
  private fun appUrl(descriptor: Descriptor): Uri =
    if (descriptor.main.startsWith("http"))
      Uri.parse(descriptor.main) // FIXME rm
    else
      Uri.Builder()
        .scheme("http")
        .encodedAuthority("localhost:${RestServer.Port}")
        // .path("apps/${descriptor.id}/${descriptor.main}
        // TODO: Remove, once we rewrite the build descriptor properly
        // https://github.com/Actyx/Cosmos/issues/3222
        .path("apps/${descriptor.id}/${descriptor.main.split("/").last()}")
        .build()

  private fun loadSettingsSchema(descriptor: Descriptor): String {
    val schemaFile = File(currentAppDir(descriptor.id), descriptor.settingsSchema)
    return schemaFile.readText()
  }

  /**
   * base app directory that contains the version directories
   */
  private fun appBaseDir(id: String): File =
    File(baseDir, id)

  /**
   * directory that contains the latest app version
   */
  private fun currentAppDir(appId: String): File? {
    val appDir = appBaseDir(appId)
    val current = currentFile(appId)
    return if (current.exists()) {
      val currentVersion = current.readText().trim()
      File(appDir, currentVersion)
    } else null
  }

  private fun currentFile(appId: String): File =
    File(appBaseDir(appId), "current")

  /**
   * loads the version-specific app descriptor
   */
  private fun descriptor(appVersionDir: File): Descriptor =
    Descriptor.parseYaml(File(appVersionDir, "ax-descriptor.yml").readText())

  /**
   * directory referenced from the descriptor that contains the distributed resources
   */
  private fun currentDistDir(appId: String): File? =
    currentAppDir(appId)?.let { current ->
      File(current, descriptor(current).dist)
    }

  private fun isDeviceLocked(): Boolean {
    // This only works when a passcode is set.
    if ((ctx.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager).isKeyguardLocked) {
      return true
    }
    return !(ctx.getSystemService(Context.POWER_SERVICE) as PowerManager).isInteractive
  }

  fun startApp(appId: String) {
    appInfo(appId)?.let { app ->
      // Starting activities when the screen is locked is problematic. Just ignore, we'll get
      // another start command from the node eventually.
      if (!isDeviceLocked()) {
        // starting an app implies enabling which is usually done by the user on the host OS
        val intent = Intent(ctx, WebappActivity::class.java).apply {
          flags = FLAG_ACTIVITY_NEW_TASK
          action = Intent.ACTION_VIEW
          setDataAndType(app.uri, MainActivity.ACTYXOS_APP_MIME_TYPE)
          putExtra(WebappActivity.EXTRA_SHORTCUT_APP_ID, app.id)
        }
        ctx.startActivity(intent)

        // trigger the actual start
        val startWebAppIntent = Intent(BackgroundServices.ACTION_APP_START_REQUESTED).apply {
          putExtra(BackgroundServices.EXTRA_APP_ID, app.id)
        }
        ctx.sendBroadcast(startWebAppIntent)
      } else {
        log.info("$appId not started as screen is currently locked")
      }
    }
  }

  fun stopApp(appId: String): Either<String, Unit> =
    Option.fromNullable(appInfo(appId))
      .toEither { "Unknown application ID '$appId'" }
      .map { app ->
        val intent = Intent(BackgroundServices.ACTION_APP_STOP_REQUESTED).apply {
          putExtra(BackgroundServices.EXTRA_APP_ID, app.id)
        }
        ctx.sendBroadcast(intent)
      }

  class ResourceNotFoundException(msg: String) : RuntimeException(msg)
}
