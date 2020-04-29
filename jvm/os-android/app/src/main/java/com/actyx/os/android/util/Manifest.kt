package com.actyx.os.android.util

import arrow.core.Option
import org.yaml.snakeyaml.Yaml

data class Manifest(
  val manifestVersion: String,
  val manifest: ManifestDetails
) {
  data class ManifestDetails(
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

    fun load(manifestYaml: String): Manifest {
      val manifest = yaml.load<Map<*, *>>(manifestYaml)
      return Manifest(
        manifestVersion = manifest["manifestVersion"] as String,
        manifest = ManifestDetails(
          id = manifest["id"] as String,
          version = manifest["version"] as String,
          name = manifest["displayName"] as String,
          description = manifest["description"] as String,
          appIconPath = Option.fromNullable(manifest["icon"] as String?),
          main = manifest["main"] as String,
          dist = manifest["dist"] as String,
          settingsSchema = manifest["settingsSchema"] as String
        )
      )
    }
  }
}
