package com.actyx.os.android.activity.systeminfoscreens

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.annotation.DrawableRes
import androidx.annotation.IdRes
import androidx.recyclerview.widget.RecyclerView
import com.actyx.os.android.R

interface IRow
class HeaderRow() : IRow
class SectionHeaderRow(val label: String) : IRow
class LabelWithValueRow(val label: String, val value: String) : IRow
class LabelWithValueRowAndIcon(
  val label: String,
  val value: String,
  @DrawableRes val iconResId: Int
) : IRow

class LinkRow(val label: String, @IdRes val navId: Int) : IRow

class HeaderViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView)

class SectionHeaderViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
  val label: TextView = itemView.findViewById(R.id.label)
}

class LabelWithValueViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
  val labelWithValue: LabelWithText = itemView.findViewById(R.id.labelWithValue)
}

class LabelWithValueAndIconViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
  val label: TextView = itemView.findViewById(R.id.label)
  val textWithIcon: TextWithIcon = itemView.findViewById(R.id.value_with_icon)
}

class LinkViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
  val label: TextView = itemView.findViewById(R.id.label)
}

class SystemInfoContentAdapter(private val rows: List<IRow>) :
  RecyclerView.Adapter<RecyclerView.ViewHolder>() {

  override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) = when (viewType) {
    TYPE_HEADER -> HeaderViewHolder(
      LayoutInflater.from(parent.context)
        .inflate(R.layout.system_info_list_header, parent, false)
    )
    TYPE_SECTION_HEADER -> SectionHeaderViewHolder(
      LayoutInflater.from(parent.context)
        .inflate(R.layout.system_info_list_row_section_header, parent, false)
    )
    TYPE_LABEL_WITH_VALUE -> LabelWithValueViewHolder(
      LayoutInflater.from(parent.context)
        .inflate(R.layout.system_info_list_row_label_with_value, parent, false)
    )
    TYPE_LABEL_WITH_VALUE_AND_ICON -> LabelWithValueAndIconViewHolder(
      LayoutInflater.from(parent.context)
        .inflate(R.layout.system_info_list_row_label_with_value_and_icon, parent, false)
    )
    TYPE_LINK -> LinkViewHolder(
      LayoutInflater.from(parent.context)
        .inflate(R.layout.system_info_list_row_link, parent, false)
    )
    else -> throw IllegalArgumentException()
  }

  override fun getItemViewType(position: Int): Int =
    when (rows[position]) {
      is HeaderRow -> TYPE_HEADER
      is SectionHeaderRow -> TYPE_SECTION_HEADER
      is LabelWithValueRow -> TYPE_LABEL_WITH_VALUE
      is LabelWithValueRowAndIcon -> TYPE_LABEL_WITH_VALUE_AND_ICON
      is LinkRow -> TYPE_LINK
      else -> throw IllegalArgumentException()
    }

  override fun onBindViewHolder(holder: RecyclerView.ViewHolder, position: Int) =
    when (holder.itemViewType) {
      TYPE_HEADER -> {
      }
      TYPE_SECTION_HEADER -> onBindSectionHeader(holder, rows[position] as SectionHeaderRow)
      TYPE_LABEL_WITH_VALUE -> onBindLabelWithValue(holder, rows[position] as LabelWithValueRow)
      TYPE_LABEL_WITH_VALUE_AND_ICON -> onBindLabelWithValueAndIcon(
        holder,
        rows[position] as LabelWithValueRowAndIcon
      )
      TYPE_LINK -> onBindLink(holder, rows[position] as LinkRow)
      else -> throw IllegalArgumentException()
    }

  override fun getItemCount() = rows.count()

  private fun onBindSectionHeader(holder: RecyclerView.ViewHolder, row: SectionHeaderRow) {
    val rowHolder = holder as SectionHeaderViewHolder
    rowHolder.label.text = row.label
  }

  private fun onBindLabelWithValue(holder: RecyclerView.ViewHolder, row: LabelWithValueRow) {
    val rowHolder = holder as LabelWithValueViewHolder
    rowHolder.labelWithValue.setLabel(row.label)
    rowHolder.labelWithValue.setValue(row.value)
  }

  private fun onBindLabelWithValueAndIcon(
    holder: RecyclerView.ViewHolder,
    row: LabelWithValueRowAndIcon
  ) {
    val rowHolder = holder as LabelWithValueAndIconViewHolder
    rowHolder.label.text = row.label
    rowHolder.textWithIcon.setText(row.value)
    rowHolder.textWithIcon.setImageResource(row.iconResId)
  }

  private fun onBindLink(holder: RecyclerView.ViewHolder, row: LinkRow) {
    val rowHolder = holder as LinkViewHolder
    rowHolder.label.text = row.label
  }

  companion object {
    private const val TYPE_HEADER = 0
    private const val TYPE_SECTION_HEADER = 1
    private const val TYPE_LABEL_WITH_VALUE = 2
    private const val TYPE_LABEL_WITH_VALUE_AND_ICON = 3
    private const val TYPE_LINK = 4
  }
}
