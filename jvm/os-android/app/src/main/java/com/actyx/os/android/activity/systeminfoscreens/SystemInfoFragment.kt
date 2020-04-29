package com.actyx.os.android.activity.systeminfoscreens

import android.os.Bundle
import androidx.fragment.app.Fragment
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.navigation.NavController
import androidx.navigation.Navigation

import com.actyx.os.android.R
import kotlinx.android.synthetic.main.fragment_system_info.view.*

class SystemInfoFragment : Fragment(), View.OnClickListener {

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
    navController = Navigation.findNavController(view)
    view.bandwidth_usage_btn.setOnClickListener(this)
    view.connected_peers_btn.setOnClickListener(this)
    view.console_btn.setOnClickListener(this)
    view.ipfs_storage_btn.setOnClickListener(this)
  }

  override fun onClick(v: View?) {
    when (v!!.id) {
      R.id.bandwidth_usage_btn -> {
        navController.navigate(R.id.action_systemInfoFragment_to_bandwidthUsageFragment)
      }
      R.id.connected_peers_btn -> {
        navController.navigate(R.id.action_systemInfoFragment_to_connectedPeersFragment)
      }
      R.id.console_btn -> {
        navController.navigate(R.id.action_systemInfoFragment_to_consoleFragment)
      }
      R.id.ipfs_storage_btn -> {
        navController.navigate(R.id.action_systemInfoFragment_to_ipfsStorageFragment)
      }
    }
  }
}
