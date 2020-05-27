package com.actyx.os.android.util

import android.annotation.SuppressLint
import com.github.davidmoten.rx2.Strings
import io.reactivex.Observable
import io.reactivex.Single
import io.reactivex.exceptions.UndeliverableException
import io.reactivex.plugins.RxJavaPlugins
import io.reactivex.schedulers.Schedulers
import org.slf4j.Logger
import java.io.IOException
import java.io.InputStream
import java.net.SocketException
import java.util.concurrent.TimeUnit

@SuppressLint("CheckResult")
fun Process.log(
  cmd: String,
  stdOut: (Observable<String>) -> Any,
  stdErr: (Observable<String>) -> Any
): Process {

  fun lineObs(inputStream: InputStream) =
    Strings
      .from(inputStream)
      .compose(Strings.split("\n"))
      .filter { it.isNotBlank() }
      .map { "$cmd: $it" }
      .toObservable()
      .subscribeOn(Schedulers.io())

  stdOut(lineObs(this.inputStream))
  stdErr(lineObs(this.errorStream))

  return this
}

fun <T> Observable<T>.retryAlways(
  onError: (Throwable) -> Unit
): Observable<T> =
  this.retryWhen { t -> t.doOnNext(onError) }

fun <T> Single<T>.retryAlways(
  onError: (Throwable) -> Unit
): Single<T> =
  this.retryWhen { t -> t.doOnNext(onError) }

fun <T> Observable<T>.mapError(f: (Throwable) -> Throwable): Observable<T> =
  this.onErrorResumeNext { e: Throwable -> Observable.error(f(e)) }

fun <T> Single<T>.mapError(f: (Throwable) -> Throwable): Single<T> =
  this.onErrorResumeNext { e: Throwable -> Single.error(f(e)) }

fun <T> Observable<T>.retryDelayed(
  delay: Long,
  unit: TimeUnit,
  onError: (Throwable) -> Unit
): Observable<T> =
  this.retryWhen { t -> t.doOnNext(onError).delay(delay, unit) }

fun <T, R> Observable<T>.collectNonNull(f: (T) -> R?): Observable<R> = flatMap {
  f(it)?.let { x -> Observable.just(x) } ?: Observable.empty()
}

object Rx {
  val log = Logger()

  /**
   * Creates a cancellable Observable from the passed process factory.
   */
  fun fromProcess(process: () -> Process, logger: Logger): Single<Unit> =
    Single.create { subscriber ->
      try {
        val p = process()
        subscriber.setCancellable(p::destroy)
        val res = p.waitFor()
        if (!subscriber.isDisposed) {
          if (res == 0) {
            subscriber.onSuccess(Unit)
          } else {
            subscriber.onError(RuntimeException("Process returned '$res'"))
          }
        }
      } catch (ie: InterruptedException) {
        logger.info("Completing interrupted process Observable.")
        subscriber.onSuccess(Unit)
      } catch (e: Exception) {
        if (!subscriber.isDisposed) {
          subscriber.onError(e)
        } else {
          logger.error("Error running process: ${e.message}, e")
        }
      }
    }

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
