package com.actyx.os.android.activity

import androidx.activity.OnBackPressedCallback
import androidx.appcompat.app.ActionBarDrawerToggle
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.Toolbar
import androidx.core.view.GravityCompat
import androidx.drawerlayout.widget.DrawerLayout
import androidx.navigation.findNavController
import androidx.navigation.ui.AppBarConfiguration
import androidx.navigation.ui.setupActionBarWithNavController
import com.actyx.os.android.R
import com.google.android.material.navigation.NavigationView

fun AppCompatActivity.setupNavigation() {
  val drawerLayout: DrawerLayout = findViewById(R.id.drawer_layout)
  val navController = findNavController(R.id.nav_host_container)

  // Configure toolbar
  val toolbar = findViewById<Toolbar>(R.id.toolbar)
  setSupportActionBar(toolbar)
  toolbar.setNavigationOnClickListener {
    if (listOf(
        R.id.systemInfoFragment,
        R.id.appsFragment
      ).contains(navController.currentDestination?.id)
    ) {
      if (!drawerLayout.isDrawerOpen(GravityCompat.START)) {
        drawerLayout.openDrawer(GravityCompat.START)
      }
    } else {
      navController.navigateUp()
    }
  }

  // Configure navigation view
  val navigationView = findViewById<NavigationView>(R.id.nav_view)
  navigationView.setNavigationItemSelectedListener { menuItem ->
    when (menuItem.itemId) {
      R.id.nav_apps -> {
        navController.navigate(R.id.appsFragment)
      }
      R.id.nav_system_info -> {
        navController.navigate(R.id.systemInfoFragment)
      }
    }

    menuItem.isChecked = true
    drawerLayout.closeDrawer(GravityCompat.START)
    true
  }
  navController.addOnDestinationChangedListener { _, destination, _ ->
    // Sync nav menu state when back button is sued
    if (listOf(
        R.id.appsFragment,
        R.id.appInfoFragment
      ).contains(destination.id)
    ) {
      navigationView.setCheckedItem(R.id.nav_apps)
    } else {
      navigationView.setCheckedItem(R.id.nav_system_info)
    }
  }

  // Sync drawer and toolbar with nav controller
  val appBarConfiguration = AppBarConfiguration
    .Builder(setOf(R.id.appsFragment, R.id.systemInfoFragment))
    .setDrawerLayout(drawerLayout)
    .build()
  setupActionBarWithNavController(navController, appBarConfiguration)
  val toggle = ActionBarDrawerToggle(
    this,
    drawerLayout,
    R.string.navigation_drawer_open,
    R.string.navigation_drawer_close
  )
  drawerLayout.addDrawerListener(toggle)
  toggle.syncState()

  // Handle back button
  this.onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
    override fun handleOnBackPressed() {
      if (drawerLayout.isDrawerOpen(GravityCompat.START)) {
        drawerLayout.closeDrawer(GravityCompat.START)
      } else {
        if (!navController.popBackStack()) {
          finish()
        }
      }
    }
  })
}
