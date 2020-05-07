import io.kotlintest.properties.Gen
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonLiteral
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject

class JsonLiteralGenerator : Gen<JsonLiteral> {
  override fun constants(): Iterable<JsonLiteral> = emptyList()

  override fun random(): Sequence<JsonLiteral> = generateSequence {
    Gen.oneOf(
      Gen.string().map { JsonLiteral(it) },
      Gen.int().map { JsonLiteral(it) },
      Gen.float().map { JsonLiteral(it) },
      Gen.double().map { JsonLiteral(it) },
      Gen.bool().map { JsonLiteral(it) }
    ).random().first()
  }
}

class JsonArrayGenerator : Gen<JsonArray> {
  override fun constants(): Iterable<JsonArray> = emptyList()

  override fun random(): Sequence<JsonArray> =
    generateSequence { JsonArray(Gen.list(JsonLiteralGenerator()).random().first()) }
}

class JsonObjectGenerator : Gen<JsonObject> {
  override fun constants(): Iterable<JsonObject> = emptyList()

  override fun random(): Sequence<JsonObject> = generateSequence {
    val size = Gen.choose(0, 100).random().first()
    val keys = Gen.string().random().take(size)

    // Avoid stack overflows
    val nestedObject =
      if (Gen.choose(0, 100).random().first() > 90)
        JsonObjectGenerator()
      else Gen.constant(JsonNull)

    val values =
      Gen.oneOf(
        JsonArrayGenerator(),
        nestedObject,
        JsonLiteralGenerator()
      )
        .random().take(size)
    JsonObject(keys.zip(values).toMap())
  }
}

class JsonElementGenerator : Gen<JsonElement> {
  override fun constants(): Iterable<JsonElement> = emptyList()

  override fun random(): Sequence<JsonElement> =
    generateSequence {
      Gen.oneOf(
        JsonLiteralGenerator(), JsonArrayGenerator(), JsonObjectGenerator(),
        Gen.constant(JsonNull)
      ).random().first()
    }
}
