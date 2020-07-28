package com.actyx.os.android.util

import io.reactivex.Observable
import io.reactivex.Single
import io.reactivex.exceptions.UndeliverableException
import io.reactivex.plugins.RxJavaPlugins
import java.io.IOException
import java.net.SocketException

fun <T> Single<T>.retryAlways(
  onError: (Throwable) -> Unit
): Single<T> =
  this.retryWhen { t -> t.doOnNext(onError) }

fun <T, R> Observable<T>.collectNonNull(f: (T) -> R?): Observable<R> = flatMap {
  f(it)?.let { x -> Observable.just(x) } ?: Observable.empty()
}

object Rx {
  val log = Logger()

  /**
   * Sets default error handlers when there's no exception handler available in the pipeline.
   *
   * This delegates to `Thread.uncaughtExceptionHandler` so make sure to set it up before calling
   * this method.
   */
  fun setErrorHandler() =
    RxJavaPlugins.setErrorHandler { e: Throwable ->
      val ee = when (e) {
        is UndeliverableException -> e.cause
        else -> e
      }

      when (ee) {
        is IOException, is SocketException -> {
          // fine, irrelevant network problem or API that throws on cancellation
        }
        is InterruptedException -> {
          // fine, some blocking code was interrupted by a dispose call
        }
        else -> {
          log.info(
            "Undeliverable exception (forwarding to uncaughtExceptionHandler): {}",
            e.message
          )
          Thread.currentThread().uncaughtExceptionHandler
            ?.uncaughtException(Thread.currentThread(), e)
        }
      }
    }
}
