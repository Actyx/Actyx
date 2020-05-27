package com.actyx.os.android.fragment

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.fragment.app.Fragment
import kotlinx.android.synthetic.main.fragment_message.view.*

import com.actyx.os.android.R

class MessageFragment : Fragment() {
  private var resId: Int = R.string.initializing

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View? {
    super.onCreateView(inflater, container, savedInstanceState)
    return inflater.inflate(R.layout.fragment_message, container, false)
  }

  override fun onActivityCreated(savedInstanceState: Bundle?) {
    super.onActivityCreated(savedInstanceState)
    view?.apply {
      icon_view.setImageDrawable(resources.getDrawable(android.R.drawable.stat_sys_warning))
    }
    update(resId)
  }

  fun update(resId: Int) {
    this.resId = resId
    view?.apply {
      text_view.text = getString(resId)
    }
  }
}
