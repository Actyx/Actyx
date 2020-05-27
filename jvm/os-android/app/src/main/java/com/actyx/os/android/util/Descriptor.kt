package com.actyx.os.android.util

import com.charleskorn.kaml.Yaml
import kotlinx.serialization.Serializable

@Serializable
data class Descriptor(
  val manifestVersion: String,
  val type: String,
  val id: String,
  val displayName: String,
  val description: String,
  val version: String,
  // relative to root
  val icon: String?,
  // relative to dist
  val main: String,
  val dist: String,
  // relative to root
  val settingsSchema: String
) {
  companion object {
    private val yaml = Yaml.default

    fun parseYaml(string: String): Descriptor =
      yaml.parse(serializer(), string)
  }
}
