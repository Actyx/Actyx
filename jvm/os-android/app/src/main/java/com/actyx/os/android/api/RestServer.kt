package com.actyx.os.android.api

import com.actyx.os.android.AppRepository
import com.actyx.os.android.util.Logger
import com.koushikdutta.async.http.AsyncHttpClient
import com.koushikdutta.async.http.server.AsyncHttpServer
import com.koushikdutta.async.http.server.AsyncHttpServerRequest
import com.koushikdutta.async.http.server.AsyncHttpServerResponse

class RestServer(private val appRepository: AppRepository) {
  private val mServer: AsyncHttpServer = AsyncHttpServer()
  private val client = AsyncHttpClient.getDefaultInstance()

  val log = Logger()

  init {
    this.mServer.get("/apps/([^/]*)/(.*)") { req, res ->
      serveFromDirectory(req, res)
    }
    this.mServer.addAction("OPTIONS", ".*") { req, res -> enableCORS(req, res) }
  }

  private fun serveFromDirectory(req: AsyncHttpServerRequest, res: AsyncHttpServerResponse) {
    val appId = req.matcher.group(1)!!
    val resourcePath = req.matcher.group(2)!!
    try {
      val (inputStream, length) =
        this.appRepository.getAppResourceAsStream(appId, resourcePath)
      res.headers.add("Access-Control-Allow-Origin", "*")
      res.sendStream(inputStream, length)
    } catch (e: AppRepository.ResourceNotFoundException) {
      res.code(404)
      res.send(e.message)
    }
  }

  private fun enableCORS(req: AsyncHttpServerRequest, res: AsyncHttpServerResponse) {
    val accessControlRequestHeaders = req.headers.get("Access-Control-Request-Headers")
    if (accessControlRequestHeaders != null) {
      res.headers.add("Access-Control-Allow-Headers", accessControlRequestHeaders)
    }

    val accessControlRequestMethod = req.headers.get("Access-Control-Request-Method")
    if (accessControlRequestMethod != null) {
      res.headers.add("Access-Control-Allow-Methods", accessControlRequestMethod)
    }
    res.send("")
  }

  fun stop() {
    this.mServer.stop()
  }

  fun start() {
    // Console Service port
    this.mServer.listen(Port)
  }

  companion object {
    const val Port = 31337
  }
}
