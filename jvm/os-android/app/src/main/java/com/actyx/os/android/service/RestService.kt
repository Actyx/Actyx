package com.actyx.os.android.service

import com.actyx.os.android.AppRepository
import com.actyx.os.android.api.RestServer
import com.actyx.os.android.util.Logger
import io.reactivex.Single

class RestService(private val appRepository: AppRepository) {

  val log = Logger()

  operator fun invoke(): Single<Unit> =
    Single.create { subscriber ->
      try {
        val server = RestServer(appRepository)
        subscriber.setCancellable {
          log.info("stopping rest service")
          server.stop()
        }
        log.info("starting rest service")
        server.start()
      } catch (e: Exception) {
        if (!subscriber.isDisposed) subscriber.onError(e)
        else log.error("error running rest server: ${e.message}", e)
      }
    }
}
