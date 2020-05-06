package com.actyx.os.android.activity.appsscreens

import android.graphics.BitmapFactory
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.actyx.os.android.AppInfo
import com.actyx.os.android.R
import kotlinx.android.extensions.LayoutContainer
import kotlinx.android.synthetic.main.app_item.view.*

class AppListAdapter(
  private val apps: Array<AppInfo>,
  private val onItemClick: (AppInfo) -> Unit
) :
  RecyclerView.Adapter<AppListAdapter.ViewHolder>() {

  override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
    val layoutInflater = LayoutInflater.from(parent.context)
    val view = layoutInflater.inflate(R.layout.app_item, parent, false)
    return ViewHolder(view, onItemClick)
  }

  override fun onBindViewHolder(holder: ViewHolder, position: Int) {
    holder.bindApp(apps[position])
  }

  override fun getItemCount(): Int = apps.size

  class ViewHolder(override val containerView: View, private val onItemClick: (AppInfo) -> Unit) :
    RecyclerView.ViewHolder(containerView), LayoutContainer {
    fun bindApp(app: AppInfo) {
      with(app) {
        containerView.firstLine.text = name
        containerView.secondLine.text = "$id v$version"
        app.iconPath?.let { containerView.icon.setImageBitmap(BitmapFactory.decodeFile(it)) }
        containerView.setOnClickListener { onItemClick(this) }
      }
    }
  }
}
