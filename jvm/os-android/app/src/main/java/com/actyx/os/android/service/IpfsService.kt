package com.actyx.os.android.service

import android.content.Context
import android.os.Build
import com.actyx.os.android.R
import com.actyx.os.android.model.ActyxOsSettings
import com.actyx.os.android.util.*
import io.reactivex.Single
import io.reactivex.schedulers.Schedulers
import kotlinx.serialization.json.booleanOrNull
import java.io.File
import java.io.InputStream
import java.util.Base64

class IpfsService(private val ctx: Context) : Service {
  private val log = Logger()

  private fun runIpfsCmd(bin: File, cmd: String, env: Array<String>): Int {
    val process = execute(bin, cmd, env)
    val res = process.waitFor()
    if (res != 0) {
      log.error("{} {} exited with {}", bin, cmd, res)
    }
    return res
  }

  /**
   * The api file is something the ipfs daemon creates at runtime and deletes on stop. However, if
   * the daemon is not properly shut down, the api hangs around and breaks things.
   *
   * See https://github.com/ipfs/go-ipfs/issues/3881#issuecomment-297863324
   */
  private fun cleanUpApiFile(ipfsPath: File) {
    val dir = File(ipfsPath, "api")
    if (dir.isFile) {
      val success = dir.delete()
      log.info("Deleted api file (success = {})", success)
    }
  }

  private fun execute(bin: File, args: String, env: Array<String>): Process {
    val cmd = "${bin.absolutePath} $args"
    log.info("""executing "{}"""", cmd)
    return Runtime.getRuntime().exec(cmd, env)
      .log(
        bin.name,
        { it.subscribe(log::info) },
        { it.subscribe(log::error) }
      )
  }

  private fun install(target: File) = Single.fromCallable {
    log.info("Installing IPFS to {}", target)
    if (!target.isDirectory) target.mkdirs()
    File(target, "ipfs").apply {
      // TODO: Somehow encode ipfs version to enable ipfs upgrades
      if (!exists()) {
        // Should theoretically be
        //
        // ipfsBinary().buffered().use { input ->
        //   outputStream().use { output ->
        //     input.copyTo(output)
        //   }
        // }
        //
        // but that fails with "java.io.IOException: Text file busy" when we try to execute `target`.
        // Might be because of https://bugs.openjdk.java.net/browse/JDK-8068370.
        ipfsBinary().buffered().use { this.writeBytes(it.readBytes()) }
        setExecutable(true)
      }
    }
  }

  private fun configure(
    ipfsBin: File,
    cfg: IpfsDetailedConfig,
    ipfsPath: File,
    ipfsEnv: Array<String>
  ) =
    Single.fromCallable {
      log.info("Configuring IPFS")
      val setupMarker = File(ipfsPath, "setup_done")
      val initial = !(setupMarker).exists()
      if (initial) ipfsPath.mkdirs()

      File(ipfsPath, "swarm.key").apply {
        log.info("Attempting to write swarm key to {}", path)
        writeText(cfg.swarmKey)
      }

      cleanUpApiFile(ipfsPath)

      if (initial) {
        log.info("Running IPFS initial setup")
        cfg.initialCalls.forEach { runIpfsCmd(ipfsBin, it, ipfsEnv) }
        setupMarker.writeText("OK")
      } else {
        log.info("Skipping initial IPFS setup as {} already exists", ipfsPath.path)
      }

      log.info("Running IPFS setup")
      cfg.restartCalls.forEach { runIpfsCmd(ipfsBin, it, ipfsEnv) }

      ipfsBin
    }

  private fun daemon(ipfsBin: File, params: List<String>, ipfsEnv: Array<String>) =
    Rx.fromProcess({
      val args = "daemon ${params.joinToString(" ")}"
      execute(ipfsBin, args, ipfsEnv)
    }, log)

  override fun invoke(settings: ActyxOsSettings): Single<Unit> {
    // Temporary feature flag to use the experimental internal ipfs node
    val usesInternalNode =
      settings.services.eventService.storeConfig?.get("ipfs_node")?.booleanOrNull ?: false
    if (usesInternalNode) {
      log.info("go-ipfs disabled")
      return Single.never()
    }
    log.info("go-ipfs enabled")
    val cfg = IpfsDetailedConfig(
      String(android.util.Base64.decode(settings.general.swarmKey, 0)),
      listOf(
        "init"
      ), listOf(
        "config Reprovider.Interval 0",
        "config Routing.Type dhtclient",
        "config --json Pubsub.Router \"gossipsub\"",
        "bootstrap rm --all",
        "config --json Pubsub.Router \"gossipsub\"",
        "config Addresses.Gateway /ip4/127.0.0.1/tcp/8080",
        "config Addresses.API /ip4/0.0.0.0/tcp/5001",
        "config Datastore.StorageMax 20GB",
        "config --json API.HTTPHeaders.Access-Control-Allow-Origin [\"*\"]",
        "config --json API.HTTPHeaders.Access-Control-Allow-Credentials [\"true\"]"
      ) + settings.general.bootstrapNodes.map { "bootstrap add $it" },
      listOf(
        "--enable-pubsub-experiment",
        "--enable-namesys-pubsub"
      ), null
    )
    val target = File(ctx.filesDir, "ipfs")
    val ipfsPath = File(ctx.getExternalFilesDir(null), "ipfs")
    val ipfsEnv = arrayOf(
      "IPFS_PATH=$ipfsPath",
      "LIBP2P_FORCE_PNET=1",
      "IPFS_LOGGING=${cfg.logLevel ?: "ERROR"}"
    )
    return install(target).mapError(::IpfsInstallationException)
      .flatMap { ipfsBin ->
        configure(ipfsBin, cfg, ipfsPath, ipfsEnv).mapError(::IpfsConfigurationException)
      }
      .flatMap { ipfsBin ->
        daemon(ipfsBin, cfg.daemonParameters, ipfsEnv).subscribeOn(Schedulers.io())
      }
  }

  private fun ipfsBinary(): InputStream {
    val arch = Build.SUPPORTED_ABIS[0] // x86_64, x86, arm64-v8a, armeabi-v7a
    log.info(
      "IPFS binary arch: {} (Supported ABIs: {})",
      arch,
      Build.SUPPORTED_ABIS.joinToString(", ")
    )
    val resourceId = when (arch) {
      "x86" -> R.raw.ipfs_x86
//      "x86_64" -> R.raw.ipfs_x86_64
      "armeabi-v7a" -> R.raw.ipfs_arm
      "arm64-v8a" -> R.raw.ipfs_arm64
      else -> 0
    }
    return ctx.resources.openRawResource(resourceId)
  }

  class IpfsInstallationException(t: Throwable) :
    RuntimeException("IPFS installation error: ${t.message}", t)

  class IpfsConfigurationException(t: Throwable) :
    RuntimeException("IPFS configuration error: ${t.message}", t)

  companion object {
    data class IpfsDetailedConfig(
      val swarmKey: String,
      // things to call ipfs cli for only on the first start. Can contain e.g. init, etc.
      val initialCalls: List<String>,
      // things to call ipfs cli for on every restart. Can contain e.g. bootstrap rm, etc.
      val restartCalls: List<String>,
      // command line arguments for the daemon. Can contain e.g. --enable-pubsub-experiment
      val daemonParameters: List<String>,
      val logLevel: String?
    )
  }
}
