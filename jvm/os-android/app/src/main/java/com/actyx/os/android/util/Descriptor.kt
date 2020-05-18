package com.actyx.os.android.util

import arrow.core.Option
import org.yaml.snakeyaml.Yaml

data class Descriptor(
  val manifestVersion: String,
  val descriptor: DescriptorDetails
) {
  data class DescriptorDetails(
    val id: String,
    val name: String,
    val description: String,
    val version: String,
    // relative to root
    val appIconPath: Option<String>,
    // relative to dist
    val main: String,
    val dist: String,
    // relative to root
    val settingsSchema: String
  )

  companion object {
    private val yaml = Yaml()

    fun load(descriptorYaml: String): Descriptor {
      val descriptor = yaml.load<Map<*, *>>(descriptorYaml)
      return Descriptor(
        manifestVersion = descriptor["manifestVersion"] as String,
        descriptor = DescriptorDetails(
          id = descriptor["id"] as String,
          version = descriptor["version"] as String,
          name = descriptor["displayName"] as String,
          description = descriptor["description"] as String,
          appIconPath = Option.fromNullable(descriptor["icon"] as String?),
          main = descriptor["main"] as String,
          dist = descriptor["dist"] as String,
          settingsSchema = descriptor["settingsSchema"] as String
        )
      )
    }
  }
}
