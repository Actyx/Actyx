package com.actyx.os.android.util

import arrow.core.some
import org.junit.Assert.*
import org.junit.Test

class DescriptorTest {
  @Test
  fun deserialize() {
    val input = """
manifestVersion: "1.0" # The version of the manifest
type: "web" # The type of app this is (web or docker)
id: "com.actyx.mwl" # A unique app id
version: "1.0.1"
displayName: "Manual Work Logging"
description: "A great first app" # A short description
icon: "./assets/app-icon.png"
dist: "./build"
main: "index.html"
settingsSchema: "./assets/schema.json"
"""

    val result = Descriptor.load(input)
    val expected = Descriptor(
      "1.0", Descriptor.DescriptorDetails(
        "com.actyx.mwl",
        "Manual Work Logging",
        "A great first app",
        "1.0.1",
        "./assets/app-icon.png".some(),
        "index.html",
        "./build",
        "./assets/schema.json"
      )
    )
    assertEquals(result, expected)
  }
}
