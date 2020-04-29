package com.actyx.os.android.legacy.usb

import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbManager
import com.felhr.usbserial.UsbSerialDevice
import com.felhr.usbserial.UsbSerialInterface
import com.actyx.os.android.util.Logger
import io.reactivex.BackpressureStrategy
import io.reactivex.Flowable
import io.reactivex.schedulers.Schedulers
import io.reactivex.subjects.PublishSubject
import io.reactivex.subjects.Subject

object IdEngine {

  val log = Logger()

  private data class Device(
    val serialDevice: UsbSerialDevice,
    val responses: Flowable<BRP.ResponseFrame>,
    val close: () -> Unit
  )

  private fun openDevice(usbManager: UsbManager, usbDevice: UsbDevice): Device {
    val connection = usbManager.openDevice(usbDevice)
    val responses: Subject<ByteArray> = PublishSubject.create()

    val serialDevice = UsbSerialDevice.createUsbSerialDevice(usbDevice, connection)
    serialDevice.open()
    serialDevice.setBaudRate(BAUDRATE)
    serialDevice.setDataBits(UsbSerialInterface.DATA_BITS_8)
    serialDevice.setParity(UsbSerialInterface.PARITY_NONE)
    serialDevice.setFlowControl(UsbSerialInterface.FLOW_CONTROL_OFF)
    serialDevice.read(responses::onNext)

    val close: () -> Unit = {
      log.info("closing {} {}.", usbDevice.productName, usbDevice.deviceName)
      serialDevice.close()
      responses.onComplete()
    }

    return Device(
      serialDevice,
      responses
        /**
         * Filter out all empty responses. Such occurs on usb card reader detach.
         * Only after some delay app gets intent that the reader was detached and
         * clean up process kicks in.
         */
        .filter { it.isNotEmpty() }
        .map(BRP::parseResponse)
        .doOnNext { log.debug("read: {}", it) }
        .toFlowable(BackpressureStrategy.ERROR),
      close
    )
  }

  private fun Flowable<BRP.CommandFrame>.usbSerialPipeline(
    device: Device
  ): Flowable<BRP.ResponseFrame> {
    val (serialDevice, responses) = device

    fun exec(command: BRP.CommandFrame): Flowable<BRP.ResponseFrame> {
      log.debug("write: cmd={}", command)
      serialDevice.write(command.toByteArray())

      return responses
        .doOnNext { res ->
          if (res.cmdCode != command.cmdCode) {
            throw RuntimeException(
              "Command code ${command.cmdCode} doesn't match response code ${res.cmdCode}.")
          }

          /**
           * On HF_ERR and NOTAG_ERR response status the flow is restarted,
           * because otherwise card reader becomes unresponsive until it gets
           * unplugged and plugged in back (which restarts the flow)
           */
          if (res.status == STATUS_HF_ERR || res.status == STATUS_NOTAG_ERR) {
            log.debug("Received status {}. Restarting flow.", BRP.hex(res.status))
            serialDevice.write(SELECT_CARD_CMD.toByteArray())
          }
        }
        .skipWhile(complement(hasStatusOkStatus()))
        .take(1)
    }

    return this.concatMap(::exec)
  }

  private fun hasStatusOkStatus() = { resp: BRP.ResponseFrame -> resp.status == STATUS_OK }
  private fun <T> complement(f: (T) -> Boolean) = { t: T -> !f(t) }

  /**
   * We use the following commands:
   *   - SELECT_CARD_CMD (Select card for further communication.)
   *   - GET_SNR_CMD (Get serial number of currently selected card.)
   * The command/response sequence is as follows:
   *    1. Send SELECT_CARD_CMD until STATUS_OK is received
   *    2. Send GET_SNR_CMD and emit the result
   *    3. Repeat
   *
   * All empty responses from the card reader are filtered.
   * On HF_ERR and NOTAG_ERR response status the flow is restarted from step 1.
   *
   *  See "VHL Reader and Driver Command Reference Version 1.25" (Vhl_1_25.pdf) and (Brp.pdf).
   *  Both pdf files could be found in google drive.
   */
  fun runFlow(usbManager: UsbManager, usbDevice: UsbDevice): Flowable<String> =
    Flowable.fromCallable { openDevice(usbManager, usbDevice) }
      .switchMap { device ->

        val cmdLoop = Flowable
          .just(SELECT_CARD_CMD, GET_SNR_CMD)
          .usbSerialPipeline(device)
          .filter { it.cmdCode == GET_SNR_CMD.cmdCode }
          .repeat()

        cmdLoop
          .map(BRP.ResponseFrame::data)
          .map { bytesToHex(it) }
          .doOnNext { x -> log.info("snr: {}", x) }
          .doOnError { e -> log.error("Baltech reader loop error ${e.message}", e) }
          .doAfterTerminate { device.close() }
          .subscribeOn(Schedulers.io())
      }

  private const val BAUDRATE = 115200
  private val SELECT_CARD_CMD = BRP.createVHLSelectCardCmd(false)
  private val GET_SNR_CMD = BRP.createVHLGetSnrCmd()
  private val STATUS_OK: Byte? = null
  // Card is not available or does not respond. This status codes is the only which requires a reselection of the card with the VHLSelect command.
  private const val STATUS_NOTAG_ERR: Byte = 0x01
  // Problems in communication with card. Data may have been corrupted.
  private const val STATUS_HF_ERR: Byte = 0x03
}
