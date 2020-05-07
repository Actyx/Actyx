package com.actyx.os.android.legacy.usb

import com.felhr.utils.HexData
import org.junit.Assert
import org.junit.Test

class BRPTest {
  @Test
  fun responseParse0() {
    // Successful read to VHLGetSnr
    val res = HexData.stringTobytes("04  01  01  04  A3  D5  D8  30  00  00  00  00")
    val parsed = BRP.parseResponse(res)
    Assert.assertEquals(
      BRP.ResponseFrame(0x04, 0x01, 0x01, 0x04, null, HexData.stringTobytes("30 D8 D5 A3"), null),
      parsed
    )
  }

  @Test
  fun responseParse1() {
    // Successful VHLSelect
    val res0 = HexData.stringTobytes("04  01  00  01  10  00  00  00")
    val parsed0 = BRP.parseResponse(res0)
    Assert.assertEquals(
      BRP.ResponseFrame(0x04, 0x01, 0x00, 0x01, null, byteArrayOf(0x10), null), parsed0
    )
  }

  @Test
  fun responseParse2() {
    // Failed VHLSelect
    val res1 = HexData.stringTobytes("84  01  01  01  02  00  00  00  00")
    val parsed1 = BRP.parseResponse(res1)
    Assert.assertEquals(
      BRP.ResponseFrame(132.toByte(), 0x01, 0x01, 0x01, 0x02, byteArrayOf(), null),
      parsed1
    )
  }
}
