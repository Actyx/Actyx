package com.actyx.os.android.util

import android.app.Activity
import android.app.Application
import android.os.Bundle

class ActivityLifecycleLogging : Application.ActivityLifecycleCallbacks {
  val log = Logger()
  override fun onActivityPaused(activity: Activity) {
    log.debug("activityLifecycle:onActivityPaused $activity")
  }

  override fun onActivityStarted(activity: Activity) {
    log.debug("activityLifecycle:onActivityStarted $activity")
  }

  override fun onActivityDestroyed(activity: Activity) {
    log.debug("activityLifecycle:onActivityDestroyed $activity")
  }

  override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) {
    log.debug("activityLifecycle:onActivitySaveInstanceState $activity, outState: $outState")
  }

  override fun onActivityStopped(activity: Activity) {
    log.debug("activityLifecycle:onActivityStopped $activity")
  }

  override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) {
    log.debug(
      "activityLifecycle:onActivityCreated $activity, savedInstanceState: $savedInstanceState"
    )
  }

  override fun onActivityResumed(activity: Activity) {
    log.debug("activityLifecycle:onActivityResumed $activity")
  }
}
