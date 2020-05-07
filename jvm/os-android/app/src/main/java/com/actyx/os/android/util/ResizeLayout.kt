package com.actyx.os.android.util

import android.content.Context
import android.graphics.Insets
import android.os.Build
import android.util.AttributeSet
import android.view.WindowInsets
import android.widget.FrameLayout

/**
 * Implements an effect similar to `android:fitsSystemWindows="true"` on Lollipop or higher,
 * except ignoring the top system window inset. `android:fitsSystemWindows="true"` does not
 * and should not be set on this layout.
 */
// https://stackoverflow.com/questions/7417123/android-how-to-adjust-layout-in-full-screen-mode-when-softkeyboard-is-visible/19494006#19494006
class ResizeLayout : FrameLayout {
  constructor(context: Context?) : super(context!!) {}
  constructor(context: Context?, attrs: AttributeSet?) : super(
    context!!,
    attrs
  )

  constructor(
    context: Context?,
    attrs: AttributeSet?,
    defStyleAttr: Int
  ) : super(context!!, attrs, defStyleAttr)

  override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
    val bottom = when (insets.systemWindowInsetBottom) {
      in 0..100 -> 0 // Use full screen, if keyboard hidden (hopefully keyboard height is > 100 px)
      else -> insets.systemWindowInsetBottom
    }
    setPadding(insets.systemWindowInsetLeft, 0, insets.systemWindowInsetRight, bottom)
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      WindowInsets.Builder(insets)
        .setSystemWindowInsets(Insets.of(0, insets.systemWindowInsetTop, 0, 0))
        .build()
    } else {
      //noinspection deprecation
      insets.replaceSystemWindowInsets(0, insets.systemWindowInsetTop, 0, 0)
    }
  }
}
