package com.actyx.os.android.activity.appsscreens

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.core.content.ContextCompat
import androidx.core.content.pm.ShortcutInfoCompat
import androidx.core.content.pm.ShortcutManagerCompat
import androidx.core.graphics.drawable.IconCompat
import androidx.fragment.app.Fragment
import com.actyx.os.android.AppInfo
import com.actyx.os.android.R
import com.actyx.os.android.activity.MainActivity
import com.actyx.os.android.activity.WebappActivity
import com.actyx.os.android.util.toBitmap
import kotlinx.android.synthetic.main.fragment_app_info.*

class AppInfoFragment : Fragment(), View.OnClickListener {

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View? =
    inflater.inflate(R.layout.fragment_app_info, container, false)

  private lateinit var appInfo: AppInfo

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    // TODO: use gradle plugin
    appInfo = requireArguments().getParcelable("appInfo")!!
  }

  override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
    super.onViewCreated(view, savedInstanceState)

    app_name.text = appInfo.name

    create_home_screen_icon_btn.setOnClickListener(this)
    open_app_btn.setOnClickListener(this)
  }

  override fun onClick(v: View?) {
    val ctx = context ?: return
    when (v!!.id) {
      R.id.create_home_screen_icon_btn -> {
        if (ShortcutManagerCompat.isRequestPinShortcutSupported(ctx)) {
          addShortcut(appInfo, ctx)
        }
      }
      R.id.open_app_btn -> {
        startActivity(mkWebappIntent(appInfo, ctx))
      }
    }
  }
}

private fun addShortcut(app: AppInfo, ctx: Context) {
  // TODO: deduplicate shortcuts
  val icon = app.iconPath?.let { BitmapFactory.decodeFile(it) } ?: ContextCompat.getDrawable(
    ctx,
    R.drawable.ic_actyxos_circle_gray
  )?.toBitmap()
  val shortcutInfo =
    // TODO revisit ID
    ShortcutInfoCompat.Builder(ctx, "${app.name}::${app.uri}")
      .setShortLabel(app.name)
      .setLongLabel(app.name)
      // only the bitmap and resource types are supported
      .setIcon(IconCompat.createWithBitmap(icon))
      .setIntent(mkWebappIntent(app, ctx))
      .build()

  val pinnedShortcutCallbackIntent = Intent(MainActivity.ACTION_SHORTCUT_ADDED_CALLBACK)
  val successCallback = PendingIntent.getBroadcast(
    ctx, MainActivity.REQ_CODE_SHORTCUT_ADDED_CALLBACK,
    pinnedShortcutCallbackIntent, 0
  )

  ShortcutManagerCompat.requestPinShortcut(ctx, shortcutInfo, successCallback.intentSender)
}

private fun mkWebappIntent(app: AppInfo, ctx: Context) =
  Intent(ctx, WebappActivity::class.java).apply {
    action = Intent.ACTION_VIEW
    setDataAndType(app.uri, MainActivity.ACTYXOS_APP_MIME_TYPE)
    putExtra(WebappActivity.EXTRA_SHORTCUT_APP_ID, app.id)
  }
