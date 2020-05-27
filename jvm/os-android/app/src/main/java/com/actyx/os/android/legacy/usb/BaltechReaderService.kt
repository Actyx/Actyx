package com.actyx.os.android.legacy.usb

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import com.actyx.os.android.util.Logger
import io.reactivex.Observable
import io.reactivex.subjects.PublishSubject
import arrow.core.Option.Companion.fromNullable
import arrow.core.filterOrOther
import com.actyx.os.android.model.ActyxOsSettings
import com.actyx.os.android.service.Service
import io.reactivex.Single
import io.reactivex.disposables.CompositeDisposable

class BaltechReaderService(private val ctx: Context) : Service {
  val log = Logger()

  val attach = PublishSubject.create<UsbDevice>()
  val detach = PublishSubject.create<UsbDevice>()
  private val disposables = CompositeDisposable()

  override fun invoke(settings: ActyxOsSettings): Single<Unit> =
    Single.create<Unit> { emitter ->
      init()
      emitter.setCancellable(::dispose)
    }

  private val permissionReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      fromNullable(intent.getParcelableExtra<UsbDevice?>(UsbManager.EXTRA_DEVICE))
        .toEither { "$ACTION_READER_PERMISSION intent received, but device is unavailable" }
        .filterOrOther(
          { intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false) },
          { "USB permission denied for ${it.deviceName}" })
        .fold({ log.warn(it) }, { device ->
          log.info("USB permission granted for {}", device.deviceName)
          attach.onNext(device)
        })
    }
  }

  private val attachReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      intent.getParcelableExtra<UsbDevice?>(UsbManager.EXTRA_DEVICE)?.let { attach.onNext(it) }
    }
  }

  private val detachReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      fromNullable(intent.getParcelableExtra<UsbDevice?>(UsbManager.EXTRA_DEVICE))
        .filter(::isBaltech)
        .fold({}, detach::onNext)
    }
  }

  private fun init() {
    ctx.registerReceiver(permissionReceiver, IntentFilter(ACTION_READER_PERMISSION))
    ctx.registerReceiver(
      attachReceiver,
      IntentFilter(BaltechEventReceiverActivity.ACTION_READER_ATTACHED)
    )
    ctx.registerReceiver(detachReceiver, IntentFilter(UsbManager.ACTION_USB_DEVICE_DETACHED))

    disposables.addAll(
      attach.subscribe {
        log.info("attached {}", it.deviceName)
        addReader(it)
      },
      detach.subscribe {
        log.info("detached {}", it.deviceName)
      })

    // Ask permission for attached devices
    val usbManager = ctx.getSystemService(Context.USB_SERVICE) as UsbManager
    usbManager.deviceList.values
      .filter(::isBaltech)
      .forEach { device ->
        // Permissions granted here are never persisted. This happens only on implicitly handled
        // `ACTION_READER_ATTACHED` i.e. when the reader is hot-plugged.
        usbManager.requestPermission(
          device,
          PendingIntent.getBroadcast(ctx, 0, Intent(ACTION_READER_PERMISSION), 0)
        )
      }
  }

  private fun addReader(device: UsbDevice) {
    val usbManager = ctx.getSystemService(Context.USB_SERVICE) as UsbManager
    disposables.add(Observable.defer {
      IdEngine
        .runFlow(usbManager, device).toObservable()
        .takeUntil(detach.filter { d -> d.deviceName == device.deviceName })
    }
      // did we miss the detach event?
      .retry { _ -> usbManager.deviceList.contains(device.deviceName) }
      .subscribe(
        { snr ->
          val broadcastIntent = Intent(ACTION_CARD_SCANNED)
          broadcastIntent.putExtra(EXTRA_SNR, snr)
          ctx.sendBroadcast(broadcastIntent)
          log.debug("Card read {}", snr)
        },
        { log.error("stopping ${device.productName} ${device.deviceName}", it) },
        { log.info("completing {} {}", device.productName, device.deviceName) }
      ))
  }

  private fun dispose() {
    ctx.unregisterReceiver(permissionReceiver)
    ctx.unregisterReceiver(attachReceiver)
    ctx.unregisterReceiver(detachReceiver)

    disposables.dispose()

    attach.onComplete()
    detach.onComplete()
  }

  companion object {
    const val ACTION_READER_PERMISSION = "com.actyx.os.hardware.usb.action.READER_PERMISSION"
    const val ACTION_CARD_SCANNED = "com.actyx.os.hardware.usb.action.CARD_SCANNED"
    const val EXTRA_SNR = "snr"

    // NB! Values below need to be in sync with `res/xml/usb_device_filter_baltech.xml`
    private const val BALTECH_VENDOR_ID = 5037
    private const val BALTECH_PRODUCT_ID = 61465
    private fun isBaltech(dev: UsbDevice) =
      dev.vendorId == BALTECH_VENDOR_ID && dev.productId == BALTECH_PRODUCT_ID
  }
}
