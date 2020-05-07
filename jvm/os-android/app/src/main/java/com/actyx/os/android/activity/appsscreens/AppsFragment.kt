package com.actyx.os.android.activity.appsscreens

import android.os.Bundle
import androidx.fragment.app.Fragment
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.core.os.bundleOf
import androidx.fragment.app.activityViewModels
import androidx.lifecycle.Observer
import androidx.navigation.Navigation
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

import com.actyx.os.android.R
import com.actyx.os.android.viewmodel.AppInfosViewModel

class AppsFragment : Fragment() {

  override fun onCreateView(
    inflater: LayoutInflater,
    container: ViewGroup?,
    savedInstanceState: Bundle?
  ): View? =
    inflater.inflate(R.layout.fragment_apps, container, false)

  override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
    super.onViewCreated(view, savedInstanceState)

    val navigate = if (resources.getBoolean(R.bool.isTablet)) {
      val c = Navigation.findNavController(view.findViewById(R.id.apps_nav_host_container))
      fun(bundle: Bundle) { c.navigate(R.id.appInfoFragment, bundle) }
    } else {
      val c = Navigation.findNavController(view)
      fun(bundle: Bundle) { c.navigate(R.id.action_appsFragment_to_appInfoFragment, bundle) }
    }

    val appsListView: RecyclerView = view.findViewById(R.id.apps_list)
    appsListView.layoutManager = LinearLayoutManager(activity)
    val appInfos: AppInfosViewModel by activityViewModels()
    appInfos.getAppInfos().observe(viewLifecycleOwner, Observer { items ->
      appsListView.adapter = AppListAdapter(items.toTypedArray()) {
        navigate(bundleOf("appInfo" to it))
      }
    })
  }
}
