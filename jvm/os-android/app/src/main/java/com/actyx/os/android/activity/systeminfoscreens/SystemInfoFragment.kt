package com.actyx.os.android.activity.systeminfoscreens

import android.content.Context.WIFI_SERVICE
import android.content.Intent
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Toast
import androidx.annotation.IdRes
import androidx.fragment.app.Fragment
import androidx.navigation.NavController
import androidx.navigation.Navigation
import androidx.recyclerview.widget.LinearLayoutManager
import com.actyx.os.android.BuildConfig
import com.actyx.os.android.R
import com.actyx.os.android.util.AndroidVersion
import com.actyx.os.android.util.BaseRecyclerViewItemTouchListener
import com.actyx.os.android.util.DeviceSerialNr
import kotlinx.android.synthetic.main.fragment_system_info.*
import kotlinx.io.ByteBuffer
import kotlinx.io.ByteOrder
import java.net.InetAddress

class SystemInfoFragment : Fragment() {

  private lateinit var navController: NavController

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View? {
    return inflater.inflate(R.layout.fragment_system_info, container, false)
  }

  override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
    super.onViewCreated(view, savedInstanceState)

    if (resources.getBoolean(R.bool.isTablet)) {
      initFull(view)
    } else {
      initCompact()
    }
  }

  private fun initFull(view: View) {
    view.findViewById<LabelWithText>(R.id.applicationAppVersionText).setValue(getAppVersion())
    view.findViewById<LabelWithText>(R.id.applicationCompatibilityText).setValue(getCompatibility())

    view.findViewById<LabelWithText>(R.id.deviceIpAddressText).setValue(getIpAddress())
    view.findViewById<LabelWithText>(R.id.deviceSerialNumber).setValue(DeviceSerialNr.get(context))
    view.findViewById<LabelWithText>(R.id.deviceSourceIdText).setValue(getDeviceSourceId())

    view.findViewById<LabelWithText>(R.id.swarmNameText).setValue(getSwarmName())

    fun setOnClickListener(@IdRes id: Int, l: () -> Unit) {
      view.findViewById<View>(id).setOnClickListener { l() }
    }

    setOnClickListener(R.id.stop_actyx_os, ::handleOnStopActyxOSClick)
    setOnClickListener(R.id.bandwidth_usage_btn) {
      handleOnNavigateClick(R.id.action_systemInfoFragment_to_bandwidthUsageFragment)
    }
    setOnClickListener(R.id.connected_peers_btn) {
      handleOnNavigateClick(R.id.action_systemInfoFragment_to_connectedPeersFragment)
    }
    setOnClickListener(R.id.console_btn) {
      handleOnNavigateClick(R.id.action_systemInfoFragment_to_consoleFragment)
    }
    setOnClickListener(R.id.ipfs_storage_btn) {
      handleOnNavigateClick(R.id.action_systemInfoFragment_to_ipfsStorageFragment)
    }
    setOnClickListener(R.id.email, ::handleOnEmailLinkClick)
    setOnClickListener(R.id.www, ::handleOnWebsiteLinkClick)
    setOnClickListener(R.id.license, ::handleOnLicenseClick)
  }

  private fun initCompact() {
    val listItems = listOf(
      HeaderRow(),
      SectionHeaderRow(getString(R.string.application)),
      LabelWithValueRowAndIcon(
        getString(R.string.contact),
        getString(R.string.contact_actyx_io),
        R.drawable.ic_email_black_24dp,
        getString(R.string.email_icon)
      ),
      LabelWithValueRowAndIcon(
        getString(R.string.website),
        getString(R.string.www_actyx_com),
        R.drawable.ic_computer_black_24dp,
        getString(R.string.website_icon)
      ),
      // TODO license
      LabelWithValueRow(getString(R.string.app_version), getAppVersion()),
      LabelWithValueRow(getString(R.string.compatibility), getCompatibility()),

      SectionHeaderRow(getString(R.string.device)),
      LabelWithValueRow(getString(R.string.ip_address), getIpAddress()),
      LabelWithValueRow(getString(R.string.serial_number), DeviceSerialNr.get(context)),
      LabelWithValueRow(getString(R.string.device_source_id), getDeviceSourceId()),

      SectionHeaderRow(getString(R.string.actyx_os)),
      LabelWithValueRow(getString(R.string.swarm_name), getSwarmName()),
      LinkRow(
        getString(R.string.connected_peers),
        R.id.action_systemInfoFragment_to_connectedPeersFragment
      ),
      LinkRow(
        getString(R.string.ipfs_statistics),
        R.id.action_systemInfoFragment_to_ipfsStorageFragment
      ),
      LinkRow(
        getString(R.string.developer_console),
        R.id.action_systemInfoFragment_to_consoleFragment
      ),
      LinkRow(
        getString(R.string.bandwidth_usage),
        R.id.action_systemInfoFragment_to_bandwidthUsageFragment
      )
    )

    val clickListener = SystemInfoRecyclerViewTouchListener(recyclerView,
      intArrayOf(R.id.navigate_next, R.id.stop_actyx_os, R.id.value_with_icon),
      object : SystemInfoRecyclerViewTouchListener.ClickListener,
        BaseRecyclerViewItemTouchListener.ClickListener {
        override fun onStopActyxOSClick(view: View, position: Int) =
          handleOnStopActyxOSClick()

        override fun onNavigateNextClick(view: View, position: Int) {
          val link = listItems[position] as LinkRow
          handleOnNavigateClick(link.navId)
        }

        override fun onLinkClick(view: View, position: Int) {
          val link = listItems[position] as LabelWithValueRowAndIcon
          if (link.value == getString(R.string.www_actyx_com)) {
            handleOnWebsiteLinkClick()
          } else {
            handleOnEmailLinkClick()
          }
        }

        override fun onClick(view: View, position: Int) {}

        override fun onLongClick(view: View, position: Int) {}
      })

    val lm = LinearLayoutManager(context)
    recyclerView?.apply {
      adapter = SystemInfoContentAdapter(listItems)
      layoutManager = lm
      addOnItemTouchListener(clickListener)
    }
  }

  private fun handleOnWebsiteLinkClick() {
    val intent = Intent(Intent.ACTION_VIEW).apply {
      data = Uri.parse("https://${getString(R.string.www_actyx_com)}")
    }
    startActivity(intent)
  }

  private fun handleOnEmailLinkClick() {
    val intent = Intent(Intent.ACTION_SENDTO).apply {
      data = Uri.parse("mailto:${getString(R.string.contact_actyx_io)}")
      putExtra(Intent.EXTRA_SUBJECT, "Feedback/Support")
    }
    startActivity(Intent.createChooser(intent, "Send feedback"))
  }

  private fun handleOnNavigateClick(@IdRes navId: Int) {
    view?.let {
      navController = Navigation.findNavController(it)
      navController.navigate(navId)
    }
  }

  // TODO:
  private fun handleOnLicenseClick() {
    view?.let {
      Toast.makeText(it.context, getString(R.string.coming_soon), Toast.LENGTH_SHORT).show()
    }
  }

  // TODO:
  private fun handleOnStopActyxOSClick() {
    view?.let {
      Toast.makeText(it.context, getString(R.string.coming_soon), Toast.LENGTH_SHORT).show()
    }
  }

  /**
   * Data getters
   */

  // TODO: get value
  private fun getSwarmName(): String {
    return getString(R.string.coming_soon)
  }

  // TODO: get value
  private fun getDeviceSourceId(): String {
    return getString(R.string.coming_soon)
  }

  private fun getCompatibility(): String {
    return AndroidVersion.versionName(BuildConfig.MIN_SDK_API_LEVEL)
      ?: getString(R.string.version_unknown)
  }

  private fun getAppVersion(): String {
    return BuildConfig.VERSION_NAME
  }

  private fun getIpAddress(): String {
    val wifiManager = activity?.applicationContext?.getSystemService(WIFI_SERVICE) as WifiManager
    return InetAddress.getByAddress(
      ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN)
        .putInt(wifiManager.connectionInfo.ipAddress).array()
    ).hostAddress
  }
}
