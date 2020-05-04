package com.actyx.os.android.activity.systeminfoscreens

import android.content.Context
import android.util.AttributeSet
import android.view.Gravity
import android.view.LayoutInflater
import android.widget.ImageView
import android.widget.LinearLayout
import androidx.annotation.DrawableRes
import com.actyx.os.android.R
import com.google.android.material.textview.MaterialTextView
import kotlinx.android.synthetic.main.view_text_with_icon.view.*

class TextWithIcon(context: Context, attrs: AttributeSet) : LinearLayout(context, attrs) {

  val textView: MaterialTextView
  val imageView: ImageView

  init {
    LayoutInflater.from(context).inflate(R.layout.view_text_with_icon, this, true)
    orientation = HORIZONTAL
    gravity = Gravity.CENTER_VERTICAL

    textView = text
    imageView = icon

    context.theme.obtainStyledAttributes(attrs, R.styleable.TextWithIcon, 0, 0).apply {
      textView.text = getString(R.styleable.TextWithIcon_android_text)
      imageView.setImageDrawable(getDrawable(R.styleable.TextWithIcon_android_src))
    }
  }

  fun setText(text: String) {
    textView.text = text
  }

  fun setImageResource(@DrawableRes resId: Int) {
    imageView.setImageResource(resId)
  }
}
