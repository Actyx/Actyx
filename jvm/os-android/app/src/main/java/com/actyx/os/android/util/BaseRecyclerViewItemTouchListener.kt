package com.actyx.os.android.util

import android.graphics.Rect
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import androidx.annotation.IdRes
import androidx.recyclerview.widget.RecyclerView

abstract class BaseRecyclerViewItemTouchListener
<out T : BaseRecyclerViewItemTouchListener.ClickListener>(
  val recycleView: RecyclerView,
  @IdRes private val specialViewIDs: IntArray?,
  protected val mClickListener: T
) : RecyclerView.OnItemTouchListener {

  private val gestureDetector: GestureDetector by lazy {
    GestureDetector(recycleView.context, object : GestureDetector.SimpleOnGestureListener() {

      override fun onSingleTapUp(event: MotionEvent): Boolean = true

      override fun onLongPress(event: MotionEvent) {
        with(recycleView) {
          val child: View? = findChildViewUnder(event.x, event.y)
          child?.let { mClickListener.onLongClick(child, getChildAdapterPosition(child)) }
        }
      }
    })
  }

  interface ClickListener {
    fun onClick(view: View, position: Int)
    fun onLongClick(view: View, position: Int)
  }

  override fun onInterceptTouchEvent(recyclerView: RecyclerView, event: MotionEvent): Boolean {
    if (gestureDetector.onTouchEvent(event)) {
      val view: View? = recyclerView.findChildViewUnder(event.x, event.y)
      view?.let {
        val listPosition = recyclerView.getChildAdapterPosition(view)
        val specialChildView: View? = findExactChild(
          view,
          event.rawX,
          event.rawY,
          getSpecialViewClickPadding()
        )
        if (listPosition != RecyclerView.NO_POSITION) {
          if (specialChildView != null) {
            onSpecialViewClick(specialChildView, listPosition)
          } else {
            mClickListener.onClick(view, listPosition)
          }
          return true
        }
      }
    }

    return false
  }

  private fun findExactChild(view: View?, x: Float, y: Float, specialViewClickPadding: Int): View? {
    if (view == null || view !is ViewGroup) {
      return view
    }

    if (specialViewIDs != null && specialViewIDs.isNotEmpty()) {
      for (specialViewId in specialViewIDs) {
        val specialView: View? = view.findViewById(specialViewId)
        specialView?.let {
          val viewBounds = Rect()
          specialView.getGlobalVisibleRect(viewBounds)
          if (x >= viewBounds.left - specialViewClickPadding &&
            x <= viewBounds.right + specialViewClickPadding &&
            y >= viewBounds.top - specialViewClickPadding &&
            y <= viewBounds.bottom + specialViewClickPadding
          ) {
            return specialView
          }
        }
      }
    }
    return null
  }

  protected abstract fun onSpecialViewClick(specialChildView: View, listPosition: Int)

  protected abstract fun getSpecialViewClickPadding(): Int

  override fun onTouchEvent(rv: RecyclerView, e: MotionEvent) {}

  override fun onRequestDisallowInterceptTouchEvent(disallowIntercept: Boolean) {}
}
