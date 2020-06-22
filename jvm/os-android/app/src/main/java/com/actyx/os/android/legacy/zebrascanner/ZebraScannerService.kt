package com.actyx.os.android.legacy.zebrascanner

import android.content.Context
import android.content.Intent
import arrow.core.toOption
import com.actyx.os.android.model.ActyxOsSettings
import com.actyx.os.android.service.Service
import com.actyx.os.android.util.Logger
import com.symbol.emdk.EMDKManager
import com.symbol.emdk.EMDKResults
import com.symbol.emdk.barcode.BarcodeManager
import com.symbol.emdk.barcode.ScanDataCollection
import com.symbol.emdk.barcode.Scanner
import com.symbol.emdk.barcode.ScannerConfig
import com.symbol.emdk.barcode.ScannerException
import com.symbol.emdk.barcode.ScannerInfo
import com.symbol.emdk.barcode.ScannerResults
import com.symbol.emdk.barcode.StatusData
import io.reactivex.Single
import io.reactivex.subjects.PublishSubject
import java.io.Closeable

/**
 * https://techdocs.zebra.com/emdk-for-android/6-4/guide/barcode_scanning_guide/
 */
class ZebraScannerImpl :
  EMDKManager.EMDKListener, BarcodeManager.ScannerConnectionListener, Scanner.DataListener,
  Scanner.StatusListener, Closeable {

  val log = Logger()

  val scans = PublishSubject.create<String>()

  private lateinit var emdkManager: EMDKManager
  private var barcodeManager: BarcodeManager? = null
  private var scanner: Scanner? = null

  override fun onConnectionChange(
    scannerInfo: ScannerInfo,
    connectionState: BarcodeManager.ConnectionState
  ) {
    log.debug("onConnectionChange, scannerInfo: $scannerInfo, connectionState: $connectionState")
    if (scannerInfo == this.scanner?.scannerInfo) {
      when (connectionState) {
        BarcodeManager.ConnectionState.CONNECTED -> {
          this.scanner?.let {
            closeScanner(it)
            initScanner(it)
          }
        }
        BarcodeManager.ConnectionState.DISCONNECTED ->
          this.scanner?.let(::closeScanner)
      }
    }
  }

  override fun onOpened(emdkManager: EMDKManager) {
    log.debug("onOpened")

    this.emdkManager = emdkManager
    // Acquire the barcode manager resources
    val barcodeManager = emdkManager.getInstance(BarcodeFeatureType) as BarcodeManager
    barcodeManager.addConnectionListener(this)
    val maybeScanner = barcodeManager.getDevice(BarcodeManager.DeviceIdentifier.DEFAULT).toOption()

    maybeScanner.fold(
      { log.info("Zebra device found (EMDK available), but no barcode scanning capability.") },
      { scanner ->
        initScanner(scanner)

        this.barcodeManager = barcodeManager
        this.scanner = scanner
      })
  }

  override fun onData(scanDataCollection: ScanDataCollection?) {
    log.debug("onData, scanDataCollection: $scanDataCollection")
    scanDataCollection?.let { sdc ->
      if (sdc.result == ScannerResults.SUCCESS) {
        sdc.scanData
          .filter { it.labelType == ScanDataCollection.LabelType.DATAMATRIX }
          .forEach {
            log.info("onData, scanData: ${it.data}")
            scans.onNext(it.data)
          }
      }
    }
  }

  override fun onStatus(statusData: StatusData) {
    log.debug("onStatus, state: ${statusData.state}, name: ${statusData.friendlyName}")
    when (statusData.state) {
      StatusData.ScannerStates.IDLE -> {
        // An attempt to use the scanner continuously and rapidly (with a delay < 100 ms between scans)
        // may cause the scanner to pause momentarily before resuming the scanning.
        // Hence add some delay (>= 100ms) before submitting the next read.
        try {
          Thread.sleep(100)
        } catch (e: InterruptedException) {
          e.printStackTrace()
        }
        this.scanner?.let(::doRead)
      }
      else -> {
      }
    }
  }

  override fun onClosed() {
    log.debug("onClosed")
    close0()
  }

  @Throws(Exception::class)
  override fun close() {
    close0()
  }

  private fun initScanner(scanner: Scanner) {
    log.info(
      "initScanner, " +
        "name: ${scanner.scannerInfo.friendlyName}, " +
        "identifier: ${scanner.scannerInfo.deviceIdentifier.name}, " +
        "modelnr: ${scanner.scannerInfo.modelNumber}"
    )
    scanner.addDataListener(this)
    scanner.addStatusListener(this)
    scanner.triggerType = Scanner.TriggerType.HARD
    try {
      log.info("Enabling scanner")
      scanner.enable()
    } catch (e: ScannerException) {
      log.error("Unable to enable the scanner", e)
    }
    setConfig(scanner)
    doRead(scanner)
  }

  private fun getAll(decoderParams: ScannerConfig.DecoderParams) =
    arrayOf<ScannerConfig.DecoderParams.BaseDecoder>(
      decoderParams.australianPostal,
      decoderParams.aztec,
      decoderParams.canadianPostal,
      decoderParams.chinese2of5,
      decoderParams.codaBar,
      decoderParams.code11,
      decoderParams.code128,
      decoderParams.code39,
      decoderParams.code93,
      decoderParams.compositeAB,
      decoderParams.compositeC,
      decoderParams.d2of5,
      decoderParams.dataMatrix,
      decoderParams.dutchPostal,
      decoderParams.ean13,
      decoderParams.ean8,
      decoderParams.gs1Databar,
      decoderParams.gs1DatabarLim,
      decoderParams.gs1DatabarExp,
      decoderParams.i2of5,
      decoderParams.japanesePostal,
      decoderParams.korean3of5,
      decoderParams.matrix2of5,
      decoderParams.maxiCode,
      decoderParams.microPDF,
      decoderParams.microQR,
      decoderParams.msi,
      decoderParams.pdf417,
      decoderParams.qrCode,
      decoderParams.signature,
      decoderParams.tlc39,
      decoderParams.triOptic39,
      decoderParams.ukPostal,
      decoderParams.upca,
      decoderParams.upce0,
      decoderParams.upce1,
      decoderParams.us4StateFics,
      decoderParams.us4State,
      decoderParams.usPlanet,
      decoderParams.usPostNet,
      decoderParams.webCode,
      decoderParams.mailMark,
      decoderParams.hanXin
    )

  private fun setConfig(scanner: Scanner) {
    try {
      log.info("Setting scanner config")
      if (scanner.isReadPending) scanner.cancelRead()
      val config: ScannerConfig = scanner.config
      config.resetToDefault(scanner)

      // disable all decoders except DataMatrix to prevent accidental reading of barcodes etc.
      // near the SKU label
      getAll(config.decoderParams).forEach { it.enabled = false }
      config.decoderParams.dataMatrix.enabled = true
      config.scanParams.decodeLEDFeedback = true
      config.scanParams.decodeLEDTime = 100
      config.scanParams.decodeHapticFeedback = true
      config.scanParams.audioStreamType = ScannerConfig.AudioStreamType.MEDIA
      config.scanParams.decodeAudioFeedbackUri = "system/media/audio/notifications/decode-short.ogg"
      config.readerParams.readerSpecific.imagerSpecific.pickList = ScannerConfig.PickList.DISABLED
      scanner.config = config
    } catch (e: ScannerException) {
      log.error("Unable to set scanner config", e)
    }
  }

  private fun doRead(scanner: Scanner) {
    log.debug(
      "doRead, " +
        "scanner: ${scanner.scannerInfo.deviceIdentifier}, " +
        "isReadPending: ${scanner.isReadPending}}"
    )
    try {
      if (!scanner.isReadPending) scanner.read()
    } catch (e: ScannerException) {
      log.error("Unable to read", e)
    }
  }

  private fun closeScanner(scanner: Scanner) {
    log.info("closeScanner, scanner: $scanner")
    // Release all the resources
    try {
      scanner.cancelRead()
      scanner.disable()
    } catch (e: ScannerException) {
      log.error("Unable to close", e)
    }
    scanner.removeDataListener(this)
    scanner.removeStatusListener(this)
  }

  private fun close0() {
    this.scanner?.let(::closeScanner)
    this.scanner = null
    barcodeManager?.removeConnectionListener(this)
    emdkManager.release(BarcodeFeatureType)

    scans.onComplete()
  }

  companion object {
    val BarcodeFeatureType = EMDKManager.FEATURE_TYPE.BARCODE
  }
}

class ZebraScannerService(private val ctx: Context) : Service {
  val log = Logger()

  override fun invoke(actyxOsSettings: ActyxOsSettings): Single<Unit> =
    Single.create { subscriber ->
      try {
        val impl = ZebraScannerImpl()
        log.info("starting Zebra scanner service")

        val scans = impl.scans.subscribe {
          log.info("code read: {}", it)
          val broadcastIntent = Intent(ACTION_CODE_SCANNED).apply {
            putExtra(EXTRA_CODE, it)
          }
          ctx.sendBroadcast(broadcastIntent)
        }

        subscriber.setCancellable {
          log.info("stopping Zebra scanner service")
          scans.dispose()
          impl.close()
        }

        val emdkResults = EMDKManager.getEMDKManager(ctx, impl)
        if (emdkResults.statusCode == EMDKResults.STATUS_CODE.SUCCESS) {
          // When this fails we don't fail the service as it would otherwise be restarted.
          log.error("Couldn't get EMDKManager (status: ${emdkResults.statusString}).")
        }
      } catch (_: NoClassDefFoundError) {
        log.info("EMDK not available. Skipping Zebra scanner service.")
      }
      Single.never<Unit>()
    }

  companion object {
    const val ACTION_CODE_SCANNED = "com.actyx.os.hardware.zebra.action.CODE_SCANNED"
    const val EXTRA_CODE = "code"
  }
}
