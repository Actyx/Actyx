package com.actyx.os.android.util

import arrow.core.Option
import arrow.core.firstOrNone
import arrow.core.some
import arrow.core.toOption
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject

fun String.asResource(): String =
  object {}.javaClass.classLoader!!.getResource(this)!!.readText()

fun JsonElement.jsonObjOpt(): Option<JsonObject> =
  when (this) {
    is JsonObject -> this.jsonObject.some()
    else -> Option.empty()
  }

fun JsonElement.lens(path: List<String>): Option<JsonElement> {
  if (path.isEmpty()) return this.some()
  return path.firstOrNone()
    .flatMap { key ->
      this.jsonObjOpt()
        .flatMap { it[key].toOption() }
        .flatMap { it.lens(path.drop(1)) }
    }
}
