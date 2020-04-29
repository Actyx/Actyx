package com.actyx.os.android.activity.systeminfoscreens

import android.os.Bundle
import androidx.fragment.app.Fragment
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup

import com.actyx.os.android.R

/**
 * A simple [Fragment] subclass.
 */
class ConnectedPeersFragment : Fragment() {

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View? {
    return inflater.inflate(R.layout.fragment_connected_peers, container, false)
  }
}
