package com.actyx.os.android.model

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonObject

@Serializable
data class LogLevels(
  val os: String,
  val apps: String
)

@Serializable
data class GeneralSettings(
  val swarmKey: String,
  val bootstrapNodes: List<String>,
  val displayName: String,
  val logLevels: LogLevels? = null
)

@Serializable
data class EventService(
  val topic: String,
  val readOnly: Boolean,
  @SerialName("_internal")
  val storeConfig: Map<String, String>? = null
)

@Serializable
data class Services(
  val eventService: EventService,
  val blobService: JsonObject? = null,
  val consoleService: JsonObject? = null,
  val webViewRuntime: JsonObject? = null
)

@Serializable
data class Licensing(
  val os: String,
  val apps: Map<String, String>
)

// TODO: remove once we got rid of go-ipfs
@Serializable
data class ActyxOsSettings(
  val general: GeneralSettings,
  val services: Services,
  val licensing: Licensing
)
