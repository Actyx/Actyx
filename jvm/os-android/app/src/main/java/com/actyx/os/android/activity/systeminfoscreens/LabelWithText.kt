package com.actyx.os.android.activity.systeminfoscreens

import android.content.Context
import android.util.AttributeSet
import android.view.Gravity
import android.view.LayoutInflater
import android.widget.LinearLayout
import com.actyx.os.android.R
import com.google.android.material.textview.MaterialTextView
import kotlinx.android.synthetic.main.view_label_with_value.view.*

class LabelWithText(context: Context, attrs: AttributeSet) : LinearLayout(context, attrs) {

  val labelTextView: MaterialTextView
  val valueTextView: MaterialTextView

  init {
    LayoutInflater.from(context).inflate(R.layout.view_label_with_value, this, true)
    orientation = HORIZONTAL
    gravity = Gravity.CENTER_VERTICAL

    labelTextView = labelText
    valueTextView = valueText

    context.theme.obtainStyledAttributes(attrs, R.styleable.LabelWithText, 0, 0).apply {
      setLabel(getString(R.styleable.LabelWithText_label))
      setValue(getString(R.styleable.LabelWithText_value))
    }
  }

  fun setLabel(text: String?) {
    labelTextView.text = text
  }

  fun setValue(text: String?) {
    valueTextView.text = text
  }
}
