package com.actyx.os.android.legacy.usb

import java.util.Arrays
import kotlin.experimental.and

// Baltech Reader Protocol, Brp.pdf, Version 1.01
object BRP {
  fun flags(b: Byte) =
    String.format("%8s", Integer.toBinaryString((b and 0xFF.toByte()).toInt())).replace(' ', '0')

  fun hex(b: Byte?) = String.format("Ox%02X", b)

  fun crc16Ccitt(bytes: ByteArray): Int {
    var crc = 0xFFFF // initial value
    val polynomial = 0x1021 // 0001 0000 0010 0001  (0, 5, 12)

    for (b in bytes) {
      for (i in 0..7) {
        val bit = b.toInt() shr 7 - i and 1 == 1
        val c15 = crc shr 15 and 1 == 1
        crc = crc shl 1
        if (c15 xor bit) crc = crc xor polynomial
      }
    }

    crc = crc and 0xffff
    return crc
  }

  data class CommandFrame(
    val cmdCtrl: Byte,
    val devCode: Byte,
    val cmdCode: Byte,
    val len: Byte,
    val param: ByteArray,
    val checksum: ByteArray?
  ) {

    fun toByteArray(): ByteArray =
      byteArrayOf(cmdCtrl, devCode) +
        cmdCode +
        len +
        param +
        (checksum ?: byteArrayOf())

    override fun toString(): String {
      return "CommandFrame(cmdCtrl=${flags(cmdCtrl)}, devCode=$devCode, cmdCode=${
        hex(cmdCode)
      }, len=$len, param=${Arrays.toString(param)}, checksum=${Arrays.toString(checksum)})"
    }
  }

  data class ResponseFrame(
    val ctrlFlags: Byte,
    val devCode: Byte,
    val cmdCode: Byte,
    val len: Byte,
    val status: Byte?,
    val data: ByteArray,
    val checksum: ByteArray?
  ) {

    override fun toString(): String {
      return "ResponseFrame(ctrlFlags=${flags(ctrlFlags)}, devCode=$devCode, cmdCode=${
        hex(cmdCode)
      }, len=${hex(len)}, status=${hex(status)}, data=${Arrays.toString(
        data
      )}, checksum=${Arrays.toString(checksum)})"
    }

    override fun equals(other: Any?): Boolean {
      if (this === other) return true
      if (javaClass != other?.javaClass) return false

      other as ResponseFrame

      if (ctrlFlags != other.ctrlFlags) return false
      if (devCode != other.devCode) return false
      if (cmdCode != other.cmdCode) return false
      if (len != other.len) return false
      if (status != other.status) return false
      if (!data.contentEquals(other.data)) return false
      if (checksum != null) {
        if (other.checksum == null) return false
        if (!checksum.contentEquals(other.checksum)) return false
      } else if (other.checksum != null) return false

      return true
    }

    override fun hashCode(): Int {
      var result: Int = ctrlFlags.toInt()
      result = 31 * result + devCode
      result = 31 * result + cmdCode
      result = 31 * result + len
      result = 31 * result + (status ?: 0)
      result = 31 * result + data.contentHashCode()
      result = 31 * result + (checksum?.contentHashCode() ?: 0)
      return result
    }
  }

  fun createCommandFrame(devCode: Byte, cmdCode: Byte, param: ByteArray): CommandFrame {
    //                    ┌┬──────── RFU
    //                    ││┌┬────── SelChk: Select the checksum algorithm which will be used for communication.
    //                    ││││         0x0: no checksum,
    //                    ││││         0x1: BCC8 checksum
    //                    ││││         0x2: CRC16 checksum (forward shifting CRC, polynomial 0x1021)
    //                    ││││         0x3: BCC16 checksum
    //                    ││││┌───── EnLen: Enable 16-bit length information in Len, otherwise only 8 bit will be used.
    //                    │││││┌──── EnDev: Enable the optional DevCode field in the command frame.
    //                    ││││││┌─── EnContMode: Enable continuous mode for command execution.
    //                    │││││││┌── EnRepMode: Enable repeat mode for command execution.
    //                    ││││││││   (Do not set EnContMode and EnRepMode at the same time, the result will be undefined.)
    //                    76543210
    val cmdCtrl: Byte = 0b00000110
    val len: Byte = param.size.toByte()
    // TODO: set checkSum algorithm in cmdCtrl before using this.
//        val checkSum: Byte =
    return CommandFrame(cmdCtrl, devCode, cmdCode, len, param, null)
  }

  fun createVHLGetSnrCmd(): CommandFrame {
    val devCode: Byte = 0x01
    val cmdCode: Byte = 0x01
    return createCommandFrame(devCode, cmdCode, byteArrayOf())
  }

  fun createVHLIsSelectedCmd(): CommandFrame {
    val devCode: Byte = 0x01
    val cmdCode: Byte = 0x04
    return createCommandFrame(devCode, cmdCode, byteArrayOf())
  }

  fun createVHLSelectCardCmd(reselect: Boolean): CommandFrame {
    val devCode: Byte = 0x01
    val cmdCode: Byte = 0x00
    val cardTypeMask: ByteArray = byteArrayOf(0x00, 0x01)
    val resel: Byte = if (reselect) 0x01 else 0x00
    val acceptConfCard: Byte = 0x00
    return createCommandFrame(
      devCode,
      cmdCode,
      cardTypeMask + byteArrayOf(resel, acceptConfCard)
    )
  }

  fun parseResponse(resp: ByteArray): ResponseFrame {
    try {
      val ctrlFlags = resp[0]
      val devCode = resp[1]
      val cmdCode = resp[2]
      val len = resp[3]
      val status: Byte? = run {
        val statusFlag: Byte = 0x80.toByte()
        when ((ctrlFlags and statusFlag) == statusFlag) {
          true -> resp[4]
          else -> null
        }
      }
      val data = run {
        when (status == null) {
          true -> resp.slice(IntRange(4, 3 + len))
          else -> resp.slice(IntRange(5, 3 + len))
        }
      }.reversed().toByteArray()
      return ResponseFrame(ctrlFlags, devCode, cmdCode, len, status, data, null)
    } catch (e: Exception) {
      throw RuntimeException("Error while trying to parse [${bytesToHex(resp)}].", e)
    }
  }
}
