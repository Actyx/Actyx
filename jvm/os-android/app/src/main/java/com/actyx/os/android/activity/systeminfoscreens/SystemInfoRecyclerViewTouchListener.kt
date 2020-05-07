package com.actyx.os.android.activity.systeminfoscreens

import android.view.View
import androidx.annotation.IdRes
import androidx.recyclerview.widget.RecyclerView
import com.actyx.os.android.R
import com.actyx.os.android.util.BaseRecyclerViewItemTouchListener

class SystemInfoRecyclerViewTouchListener(
  recycleView: RecyclerView,
  @IdRes specialIds: IntArray?,
  clickListener: ClickListener
) : BaseRecyclerViewItemTouchListener<SystemInfoRecyclerViewTouchListener.ClickListener>(
  recycleView,
  specialIds,
  clickListener
) {
  interface ClickListener : BaseRecyclerViewItemTouchListener.ClickListener {
    fun onStopActyxOSClick(view: View, position: Int)
    fun onNavigateNextClick(view: View, position: Int)
    fun onLinkClick(view: View, position: Int)
  }

  private var clickPadding: Int

  init {
    clickPadding =
      (SPECIAL_VIEW_CLICK_AREA_EXTENSION * recycleView.resources.displayMetrics.density).toInt()
  }

  override fun onSpecialViewClick(specialChildView: View, listPosition: Int) {
    when (specialChildView.id) {
      R.id.stop_actyx_os -> mClickListener.onStopActyxOSClick(specialChildView, listPosition)
      R.id.navigate_next -> mClickListener.onNavigateNextClick(specialChildView, listPosition)
      R.id.value_with_icon -> mClickListener.onLinkClick(specialChildView, listPosition)
      else -> mClickListener.onClick(specialChildView, listPosition)
    }
  }

  override fun getSpecialViewClickPadding(): Int = clickPadding

  companion object {
    private const val SPECIAL_VIEW_CLICK_AREA_EXTENSION = 5
  }
}
